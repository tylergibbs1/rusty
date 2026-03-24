import { query, tool, createSdkMcpServer } from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";
import { execFile } from "child_process";
import { promisify } from "util";
import path from "path";
import { fileURLToPath } from "url";

const exec = promisify(execFile);

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const RUSTY_BIN = path.resolve(__dirname, "../../target/release/rusty");
const RUSTY_HOME = path.resolve(__dirname, "../../.rusty-agent-test");
const PLUGIN_DIR = path.resolve(
  __dirname,
  "../../examples/plugins/hello-world"
);

async function rusty(...args) {
  const { stdout, stderr } = await exec(RUSTY_BIN, args, {
    env: { ...process.env, RUSTY_HOME },
  });
  return { stdout: stdout.trim(), stderr: stderr.trim() };
}

// Create an MCP server that wraps rusty plugin actions
const rustyServer = createSdkMcpServer({
  name: "rusty-plugins",
  version: "0.1.0",
  tools: [
    tool(
      "rusty_install",
      "Install a WASM plugin into the rusty runtime from a local directory path",
      { path: z.string().describe("Absolute path to the plugin directory") },
      async (args) => {
        try {
          const { stdout } = await rusty("install", args.path);
          return {
            content: [{ type: "text", text: stdout }],
          };
        } catch (e) {
          return {
            content: [{ type: "text", text: `Install failed: ${e.stderr || e.message}` }],
            isError: true,
          };
        }
      },
      { annotations: { readOnly: false, destructive: false } }
    ),

    tool(
      "rusty_list",
      "List all installed WASM plugins and their available actions",
      {},
      async () => {
        try {
          const { stdout } = await rusty("list");
          return {
            content: [{ type: "text", text: stdout }],
          };
        } catch (e) {
          return {
            content: [{ type: "text", text: `List failed: ${e.stderr || e.message}` }],
            isError: true,
          };
        }
      },
      { annotations: { readOnly: true } }
    ),

    tool(
      "rusty_inspect",
      "Inspect a plugin showing its manifest, capabilities, and action schemas",
      { plugin_id: z.string().describe("The plugin ID to inspect") },
      async (args) => {
        try {
          const { stdout } = await rusty("inspect", args.plugin_id);
          return {
            content: [{ type: "text", text: stdout }],
          };
        } catch (e) {
          return {
            content: [
              { type: "text", text: `Inspect failed: ${e.stderr || e.message}` },
            ],
            isError: true,
          };
        }
      },
      { annotations: { readOnly: true } }
    ),

    tool(
      "rusty_invoke",
      "Invoke a plugin action with JSON input. Returns the action result and execution trace.",
      {
        plugin_id: z.string().describe("The plugin ID"),
        action_id: z.string().describe("The action ID to invoke"),
        input: z
          .string()
          .describe("JSON string of input parameters for the action"),
      },
      async (args) => {
        try {
          const { stdout } = await rusty(
            "invoke",
            args.plugin_id,
            args.action_id,
            "--input",
            args.input,
            "--trace"
          );
          return {
            content: [{ type: "text", text: stdout }],
          };
        } catch (e) {
          return {
            content: [
              { type: "text", text: `Invoke failed: ${e.stderr || e.message}` },
            ],
            isError: true,
          };
        }
      },
      { annotations: { readOnly: false, destructive: false } }
    ),
  ],
});

// Run the agent test
async function main() {
  console.log("=== Rusty + Agent SDK Integration Test ===\n");

  // Pre-install the plugin so it's ready
  console.log("Pre-installing hello-world plugin...");
  try {
    await rusty("install", PLUGIN_DIR);
    console.log("Plugin installed.\n");
  } catch (e) {
    console.log("Plugin may already be installed, continuing...\n");
  }

  console.log("Launching Claude agent with rusty MCP tools...\n");
  console.log("-------------------------------------------\n");

  const response = query({
    prompt: [
      "You have access to a WASM plugin runtime called 'rusty'.",
      "Please do the following steps:",
      "1. List the installed plugins to see what's available",
      "2. Inspect the hello-world plugin to understand its actions and schemas",
      "3. Invoke the 'greet' action with the name 'Claude Agent SDK'",
      "4. Try invoking greet with invalid input (empty object {}) to see schema validation",
      "5. Summarize what happened in each step",
    ].join("\n"),
    options: {
      mcpServers: { rusty: rustyServer },
      allowedTools: [
        "mcp__rusty__rusty_list",
        "mcp__rusty__rusty_inspect",
        "mcp__rusty__rusty_invoke",
        "mcp__rusty__rusty_install",
      ],
      tools: [], // disable built-in tools, only use our MCP tools
      maxTurns: 12,
    },
  });

  let resultText = "";

  for await (const message of response) {
    // Print assistant text as it arrives
    if (message.type === "assistant" && message.message?.content) {
      for (const block of message.message.content) {
        if (block.type === "text") {
          process.stdout.write(block.text);
          resultText += block.text;
        }
        if (block.type === "tool_use") {
          console.log(`\n[Tool Call: ${block.name}]`);
          console.log(`  Input: ${JSON.stringify(block.input)}`);
        }
      }
    }

    // Print tool results
    if (message.type === "result") {
      console.log("\n\n--- Agent Complete ---");
      console.log(`Session: ${message.session_id}`);
      console.log(`Cost: input=${message.usage?.input_tokens || "?"}, output=${message.usage?.output_tokens || "?"} tokens`);
    }
  }

  console.log("\n=== Test Complete ===");
}

main().catch((err) => {
  console.error("Test failed:", err);
  process.exit(1);
});
