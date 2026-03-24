use crate::error::SchemaError;

pub fn validate(
    schema: &serde_json::Value,
    instance: &serde_json::Value,
) -> Result<(), SchemaError> {
    let validator = jsonschema::options()
        .build(schema)
        .map_err(|e| SchemaError::InvalidSchema(e.to_string()))?;

    if validator.is_valid(instance) {
        Ok(())
    } else {
        let errors: Vec<String> = validator
            .iter_errors(instance)
            .map(|e| e.to_string())
            .collect();
        Err(SchemaError::ValidationFailed(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valid_instance_passes() {
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": { "type": "string" }
            }
        });
        let instance = json!({ "name": "Alice" });
        assert!(validate(&schema, &instance).is_ok());
    }

    #[test]
    fn missing_required_field_fails() {
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": { "type": "string" }
            }
        });
        let instance = json!({});
        assert!(validate(&schema, &instance).is_err());
    }

    #[test]
    fn wrong_type_fails() {
        let schema = json!({ "type": "string" });
        let instance = json!(42);
        assert!(validate(&schema, &instance).is_err());
    }

    #[test]
    fn permissive_schema_passes_anything() {
        let schema = json!({});
        let instance = json!({ "anything": [1, 2, 3] });
        assert!(validate(&schema, &instance).is_ok());
    }
}
