use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn expect_args(function: &str, args: &[RuntimeValue], expected: usize) -> Result<(), HostError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "{} expects exactly {} argument{}, found {}",
            function,
            expected,
            if expected == 1 { "" } else { "s" },
            args.len()
        )))
    }
}

fn extract_string<'a>(
    function: &str,
    args: &'a [RuntimeValue],
    idx: usize,
) -> Result<&'a str, HostError> {
    match &args[idx] {
        RuntimeValue::String(s) => Ok(s.as_str()),
        other => Err(HostError::new(format!(
            "{} expects a string argument at position {}, found {}",
            function,
            idx + 1,
            host_value_kind(other)
        ))),
    }
}

// --- RFC 4180 CSV parser ---

/// Parse a CSV string into a list of rows (each row is a list of string fields).
fn parse_csv(input: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        if in_quotes {
            if c == '"' {
                if i + 1 < len && chars[i + 1] == '"' {
                    // Escaped quote
                    field.push('"');
                    i += 2;
                } else {
                    // End of quoted field
                    in_quotes = false;
                    i += 1;
                }
            } else {
                field.push(c);
                i += 1;
            }
        } else {
            if c == '"' && field.is_empty() {
                in_quotes = true;
                i += 1;
            } else if c == ',' {
                row.push(std::mem::take(&mut field));
                i += 1;
            } else if c == '\r' {
                // CRLF or bare CR
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                if i + 1 < len && chars[i + 1] == '\n' {
                    i += 2;
                } else {
                    i += 1;
                }
            } else if c == '\n' {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                i += 1;
            } else {
                field.push(c);
                i += 1;
            }
        }
    }

    // Handle last field/row (if input doesn't end with newline)
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }

    if in_quotes {
        return Err("unterminated quoted field".to_string());
    }

    Ok(rows)
}

/// Encode a field, quoting if necessary.
fn encode_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        field.to_string()
    }
}

/// Csv.decode/1 — parse CSV string → {:ok, rows} or {:error, reason}
fn host_csv_decode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_args("Csv.decode", args, 1)?;
    let input = extract_string("Csv.decode", args, 0)?;

    match parse_csv(input) {
        Ok(rows) => {
            let rv_rows: Vec<RuntimeValue> = rows
                .into_iter()
                .map(|row| RuntimeValue::List(row.into_iter().map(RuntimeValue::String).collect()))
                .collect();
            Ok(RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("ok".to_string())),
                Box::new(RuntimeValue::List(rv_rows)),
            ))
        }
        Err(e) => Ok(RuntimeValue::Tuple(
            Box::new(RuntimeValue::Atom("error".to_string())),
            Box::new(RuntimeValue::String(e)),
        )),
    }
}

/// Csv.encode/1 — encode list of lists → CSV string
fn host_csv_encode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_args("Csv.encode", args, 1)?;
    let rows = match &args[0] {
        RuntimeValue::List(rows) => rows,
        other => {
            return Err(HostError::new(format!(
                "Csv.encode expects a list of lists, found {}",
                host_value_kind(other)
            )));
        }
    };

    let mut output = String::new();
    for (ri, row) in rows.iter().enumerate() {
        let fields = match row {
            RuntimeValue::List(fields) => fields,
            other => {
                return Err(HostError::new(format!(
                    "Csv.encode expects each row to be a list, found {} at row {}",
                    host_value_kind(other),
                    ri
                )));
            }
        };

        for (fi, field) in fields.iter().enumerate() {
            let s = runtime_to_string(field);
            if fi > 0 {
                output.push(',');
            }
            output.push_str(&encode_field(&s));
        }
        output.push('\n');
    }

    Ok(RuntimeValue::String(output))
}

/// Csv.decode_maps/1 — parse CSV with header row → {:ok, list of maps}
fn host_csv_decode_maps(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_args("Csv.decode_maps", args, 1)?;
    let input = extract_string("Csv.decode_maps", args, 0)?;

    match parse_csv(input) {
        Ok(rows) => {
            if rows.is_empty() {
                return Ok(RuntimeValue::Tuple(
                    Box::new(RuntimeValue::Atom("ok".to_string())),
                    Box::new(RuntimeValue::List(vec![])),
                ));
            }

            let headers = &rows[0];
            let mut maps: Vec<RuntimeValue> = Vec::new();

            for row in rows.iter().skip(1) {
                let mut entries: Vec<(RuntimeValue, RuntimeValue)> = Vec::new();
                for (i, header) in headers.iter().enumerate() {
                    let value = if i < row.len() {
                        RuntimeValue::String(row[i].clone())
                    } else {
                        RuntimeValue::String(String::new())
                    };
                    entries.push((RuntimeValue::String(header.clone()), value));
                }
                maps.push(RuntimeValue::Map(entries));
            }

            Ok(RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("ok".to_string())),
                Box::new(RuntimeValue::List(maps)),
            ))
        }
        Err(e) => Ok(RuntimeValue::Tuple(
            Box::new(RuntimeValue::Atom("error".to_string())),
            Box::new(RuntimeValue::String(e)),
        )),
    }
}

/// Csv.encode_maps/2 — encode list of maps with header list → CSV string
fn host_csv_encode_maps(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_args("Csv.encode_maps", args, 2)?;

    let headers = match &args[0] {
        RuntimeValue::List(h) => h,
        other => {
            return Err(HostError::new(format!(
                "Csv.encode_maps expects headers as a list, found {}",
                host_value_kind(other)
            )));
        }
    };
    let maps = match &args[1] {
        RuntimeValue::List(m) => m,
        other => {
            return Err(HostError::new(format!(
                "Csv.encode_maps expects maps as a list, found {}",
                host_value_kind(other)
            )));
        }
    };

    // Extract header strings
    let header_strs: Vec<String> = headers.iter().map(|h| runtime_to_string(h)).collect();

    let mut output = String::new();

    // Write header row
    for (i, h) in header_strs.iter().enumerate() {
        if i > 0 {
            output.push(',');
        }
        output.push_str(&encode_field(h));
    }
    output.push('\n');

    // Write data rows
    for (ri, map_val) in maps.iter().enumerate() {
        let entries = match map_val {
            RuntimeValue::Map(entries) => entries,
            other => {
                return Err(HostError::new(format!(
                    "Csv.encode_maps expects each element to be a map, found {} at row {}",
                    host_value_kind(other),
                    ri
                )));
            }
        };

        for (fi, header) in header_strs.iter().enumerate() {
            if fi > 0 {
                output.push(',');
            }
            // Find the value for this header key in the map
            let value = entries
                .iter()
                .find(|(k, _)| match k {
                    RuntimeValue::String(s) => s == header,
                    RuntimeValue::Atom(a) => a == header,
                    _ => false,
                })
                .map(|(_, v)| runtime_to_string(v))
                .unwrap_or_default();
            output.push_str(&encode_field(&value));
        }
        output.push('\n');
    }

    Ok(RuntimeValue::String(output))
}

/// Convert a RuntimeValue to a string representation for CSV encoding.
fn runtime_to_string(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::String(s) => s.clone(),
        RuntimeValue::Int(n) => n.to_string(),
        RuntimeValue::Float(s) => s.clone(),
        RuntimeValue::Bool(b) => b.to_string(),
        RuntimeValue::Atom(a) => a.clone(),
        RuntimeValue::Nil => String::new(),
        _ => format!("{:?}", value),
    }
}

pub fn register_csv_host_functions(registry: &HostRegistry) {
    registry.register("csv_decode", host_csv_decode);
    registry.register("csv_encode", host_csv_encode);
    registry.register("csv_decode_maps", host_csv_decode_maps);
    registry.register("csv_encode_maps", host_csv_encode_maps);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    fn list(items: Vec<RuntimeValue>) -> RuntimeValue {
        RuntimeValue::List(items)
    }

    fn ok_tuple(val: RuntimeValue) -> RuntimeValue {
        RuntimeValue::Tuple(
            Box::new(RuntimeValue::Atom("ok".to_string())),
            Box::new(val),
        )
    }

    fn err_tuple(msg: &str) -> RuntimeValue {
        RuntimeValue::Tuple(
            Box::new(RuntimeValue::Atom("error".to_string())),
            Box::new(s(msg)),
        )
    }

    #[test]
    fn decode_simple() {
        let result = HOST_REGISTRY
            .call("csv_decode", &[s("a,b,c\n1,2,3\n")])
            .unwrap();
        assert_eq!(
            result,
            ok_tuple(list(vec![
                list(vec![s("a"), s("b"), s("c")]),
                list(vec![s("1"), s("2"), s("3")]),
            ]))
        );
    }

    #[test]
    fn decode_quoted_fields() {
        let result = HOST_REGISTRY
            .call("csv_decode", &[s("name,desc\n\"Alice\",\"has, commas\"\n")])
            .unwrap();
        assert_eq!(
            result,
            ok_tuple(list(vec![
                list(vec![s("name"), s("desc")]),
                list(vec![s("Alice"), s("has, commas")]),
            ]))
        );
    }

    #[test]
    fn decode_escaped_quotes() {
        let result = HOST_REGISTRY
            .call("csv_decode", &[s("val\n\"say \"\"hello\"\"\"\n")])
            .unwrap();
        assert_eq!(
            result,
            ok_tuple(list(vec![
                list(vec![s("val")]),
                list(vec![s("say \"hello\"")]),
            ]))
        );
    }

    #[test]
    fn decode_multiline_field() {
        let result = HOST_REGISTRY
            .call("csv_decode", &[s("a,b\n\"line1\nline2\",val\n")])
            .unwrap();
        assert_eq!(
            result,
            ok_tuple(list(vec![
                list(vec![s("a"), s("b")]),
                list(vec![s("line1\nline2"), s("val")]),
            ]))
        );
    }

    #[test]
    fn decode_crlf() {
        let result = HOST_REGISTRY
            .call("csv_decode", &[s("a,b\r\n1,2\r\n")])
            .unwrap();
        assert_eq!(
            result,
            ok_tuple(list(vec![
                list(vec![s("a"), s("b")]),
                list(vec![s("1"), s("2")]),
            ]))
        );
    }

    #[test]
    fn decode_empty_input() {
        let result = HOST_REGISTRY.call("csv_decode", &[s("")]).unwrap();
        assert_eq!(result, ok_tuple(list(vec![])));
    }

    #[test]
    fn decode_unterminated_quote() {
        let result = HOST_REGISTRY
            .call("csv_decode", &[s("\"unterminated")])
            .unwrap();
        assert_eq!(result, err_tuple("unterminated quoted field"));
    }

    #[test]
    fn encode_simple() {
        let result = HOST_REGISTRY
            .call(
                "csv_encode",
                &[list(vec![
                    list(vec![s("a"), s("b")]),
                    list(vec![s("1"), s("2")]),
                ])],
            )
            .unwrap();
        assert_eq!(result, s("a,b\n1,2\n"));
    }

    #[test]
    fn encode_auto_quoting() {
        let result = HOST_REGISTRY
            .call(
                "csv_encode",
                &[list(vec![list(vec![s("has, comma"), s("has \"quote\"")])])],
            )
            .unwrap();
        assert_eq!(result, s("\"has, comma\",\"has \"\"quote\"\"\"\n"));
    }

    #[test]
    fn encode_round_trip() {
        let original = "name,desc\nAlice,\"likes, commas\"\nBob,\"says \"\"hi\"\"\"\n";
        let decoded = HOST_REGISTRY.call("csv_decode", &[s(original)]).unwrap();
        match decoded {
            RuntimeValue::Tuple(_, rows) => {
                let encoded = HOST_REGISTRY.call("csv_encode", &[*rows]).unwrap();
                assert_eq!(encoded, s(original));
            }
            other => panic!("expected tuple, got {:?}", other),
        }
    }

    #[test]
    fn decode_maps_simple() {
        let result = HOST_REGISTRY
            .call("csv_decode_maps", &[s("name,age\nAlice,30\nBob,25\n")])
            .unwrap();
        match result {
            RuntimeValue::Tuple(tag, val) => {
                assert_eq!(*tag, RuntimeValue::Atom("ok".to_string()));
                match *val {
                    RuntimeValue::List(maps) => {
                        assert_eq!(maps.len(), 2);
                        // Check first map has name=Alice, age=30
                        match &maps[0] {
                            RuntimeValue::Map(entries) => {
                                assert_eq!(entries.len(), 2);
                                assert!(entries.contains(&(s("name"), s("Alice"))));
                                assert!(entries.contains(&(s("age"), s("30"))));
                            }
                            other => panic!("expected map, got {:?}", other),
                        }
                    }
                    other => panic!("expected list, got {:?}", other),
                }
            }
            other => panic!("expected tuple, got {:?}", other),
        }
    }

    #[test]
    fn decode_maps_empty() {
        let result = HOST_REGISTRY.call("csv_decode_maps", &[s("")]).unwrap();
        assert_eq!(result, ok_tuple(list(vec![])));
    }

    #[test]
    fn decode_maps_header_only() {
        let result = HOST_REGISTRY
            .call("csv_decode_maps", &[s("name,age\n")])
            .unwrap();
        assert_eq!(result, ok_tuple(list(vec![])));
    }

    #[test]
    fn encode_maps_simple() {
        let headers = list(vec![s("name"), s("age")]);
        let maps = list(vec![
            RuntimeValue::Map(vec![(s("name"), s("Alice")), (s("age"), s("30"))]),
            RuntimeValue::Map(vec![(s("name"), s("Bob")), (s("age"), s("25"))]),
        ]);
        let result = HOST_REGISTRY
            .call("csv_encode_maps", &[headers, maps])
            .unwrap();
        assert_eq!(result, s("name,age\nAlice,30\nBob,25\n"));
    }

    #[test]
    fn decode_arity_rejection() {
        assert!(HOST_REGISTRY.call("csv_decode", &[]).is_err());
        assert!(HOST_REGISTRY.call("csv_decode", &[s("a"), s("b")]).is_err());
    }

    #[test]
    fn encode_non_list_rejection() {
        assert!(HOST_REGISTRY
            .call("csv_encode", &[s("not a list")])
            .is_err());
    }

    #[test]
    fn registration() {
        assert!(HOST_REGISTRY.call("csv_decode", &[s("")]).is_ok());
        assert!(HOST_REGISTRY.call("csv_encode", &[list(vec![])]).is_ok());
        assert!(HOST_REGISTRY.call("csv_decode_maps", &[s("")]).is_ok());
        assert!(HOST_REGISTRY
            .call("csv_encode_maps", &[list(vec![]), list(vec![])])
            .is_ok());
    }
}
