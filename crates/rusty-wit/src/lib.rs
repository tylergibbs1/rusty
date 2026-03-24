wasmtime::component::bindgen!({
    inline: r#"
        package rusty:plugin@0.1.0;

        interface types {
            enum log-level {
                trace,
                debug,
                info,
                warn,
                error,
            }

            enum action-kind {
                read-only,
                mutating,
                destructive,
            }

            enum approval-class {
                none-required,
                auto-approve,
                require-human,
            }

            record action-def {
                id: string,
                title: string,
                description: string,
                input-schema: string,
                output-schema: string,
                kind: action-kind,
                approval: approval-class,
                tags: list<string>,
            }

            record plugin-info {
                id: string,
                name: string,
                version: string,
                author: string,
                description: string,
            }

            record action-error {
                code: string,
                message: string,
                details: option<string>,
            }

            variant action-result {
                ok(string),
                err(action-error),
            }
        }

        interface host-api {
            use types.{log-level};

            log: func(level: log-level, message: string);
            get-config: func(key: string) -> option<string>;
            emit-event: func(event-type: string, payload: string);
        }

        interface guest {
            use types.{plugin-info, action-def, action-result};

            get-info: func() -> plugin-info;
            list-actions: func() -> list<action-def>;
            invoke: func(action-id: string, input: string) -> action-result;
        }

        world plugin-world {
            import host-api;
            export guest;
        }
    "#,
    imports: { default: async },
    exports: { default: async },
});

pub use rusty::plugin::host_api;
pub use rusty::plugin::types;
