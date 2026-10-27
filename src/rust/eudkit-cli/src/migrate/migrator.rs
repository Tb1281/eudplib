use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Whitespace(String),
    Comment(String),
    StringLit(String),
    Number(String),
    Ident(String),
    Symbol(String),
}

impl Token {
    pub fn as_str(&self) -> &str {
        match self {
            Token::Whitespace(s)
            | Token::Comment(s)
            | Token::StringLit(s)
            | Token::Number(s)
            | Token::Ident(s)
            | Token::Symbol(s) => s.as_str(),
        }
    }

    pub fn is_trivia(&self) -> bool {
        matches!(self, Token::Whitespace(_) | Token::Comment(_))
    }
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // Whitespace
        if ch.is_whitespace() {
            let start = i;
            while i < len && chars[i].is_whitespace() {
                i += 1;
            }
            tokens.push(Token::Whitespace(chars[start..i].iter().collect()));
            continue;
        }

        // Line comment
        if ch == '/' && i + 1 < len && chars[i + 1] == '/' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            tokens.push(Token::Comment(chars[start..i].iter().collect()));
            continue;
        }

        // Block comment
        if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            } else {
                i = len;
            }
            tokens.push(Token::Comment(chars[start..i].iter().collect()));
            continue;
        }

        // Strings
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let start = i;
            i += 1;
            let mut escaped = false;
            while i < len {
                let c = chars[i];
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            tokens.push(Token::StringLit(chars[start..i].iter().collect()));
            continue;
        }

        // Dollar macro (e.g. $U, $P8, $B, $T, $L, $S)
        if ch == '$' {
            let start = i;
            i += 1;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token::Ident(chars[start..i].iter().collect()));
            continue;
        }

        // Numbers (hex, binary, decimal)
        if ch.is_ascii_digit() {
            let start = i;
            if ch == '0' && i + 1 < len && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
                i += 2;
                while i < len && chars[i].is_ascii_hexdigit() {
                    i += 1;
                }
            } else if ch == '0' && i + 1 < len && (chars[i + 1] == 'b' || chars[i + 1] == 'B') {
                i += 2;
                while i < len && (chars[i] == '0' || chars[i] == '1') {
                    i += 1;
                }
            } else {
                while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
            }
            tokens.push(Token::Number(chars[start..i].iter().collect()));
            continue;
        }

        // Identifiers & keywords
        if ch.is_alphabetic() || ch == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token::Ident(chars[start..i].iter().collect()));
            continue;
        }

        // Multi-character symbols
        if i + 2 < len {
            let tri: String = chars[i..i + 3].iter().collect();
            if matches!(tri.as_str(), "<<=" | ">>=") {
                tokens.push(Token::Symbol(tri));
                i += 3;
                continue;
            }
        }
        if i + 1 < len {
            let duo: String = chars[i..i + 2].iter().collect();
            if matches!(
                duo.as_str(),
                "==" | "!="
                    | "<="
                    | ">="
                    | "+="
                    | "-="
                    | "*="
                    | "/="
                    | "%="
                    | "&="
                    | "|="
                    | "^="
                    | "<<"
                    | ">>"
                    | "&&"
                    | "||"
                    | "++"
                    | "--"
                    | "::"
            ) {
                tokens.push(Token::Symbol(duo));
                i += 2;
                continue;
            }
        }

        // Single-character symbol
        tokens.push(Token::Symbol(ch.to_string()));
        i += 1;
    }

    tokens
}

pub fn convert_eps_to_ts(source: &str) -> Result<String, String> {
    let tokens = tokenize(source);
    let mut out = String::with_capacity(source.len() + 512);

    // Track state
    let mut in_object = false;
    let mut in_for_header = false;
    let mut for_paren_depth = 0;
    let mut has_main = false;
    let mut once_counter = 0;

    let mut declared_arrays: HashSet<String> = HashSet::new();

    let mut i = 0;
    let len = tokens.len();

    // Helper closures
    let next_non_trivia = |start: usize| -> Option<usize> {
        let mut idx = start;
        while idx < len {
            if !tokens[idx].is_trivia() {
                return Some(idx);
            }
            idx += 1;
        }
        None
    };

    while i < len {
        let tok = &tokens[i];

        match tok {
            Token::Whitespace(s) | Token::Comment(s) => {
                out.push_str(s);
                i += 1;
            }

            Token::Ident(ident) => {
                let id_str = ident.as_str();

                // Macro replacements ($P1..$P12, $U, $B, $T, $L, $S)
                if let Some(num_part) = id_str.strip_prefix("$P") {
                    if let Ok(num) = num_part.parse::<u32>() {
                        if (1..=12).contains(&num) {
                            out.push_str(&format!("Player{num}"));
                            i += 1;
                            continue;
                        }
                    }
                }

                if matches!(id_str, "$U" | "$B" | "$T" | "$L" | "$S") {
                    // Check if followed by `(`
                    if let Some(open_p) = next_non_trivia(i + 1) {
                        if tokens[open_p].as_str() == "(" {
                            // Find matching closing paren
                            if let Some(inner) = next_non_trivia(open_p + 1) {
                                if let Some(close_p) = next_non_trivia(inner + 1) {
                                    if tokens[close_p].as_str() == ")" {
                                        out.push_str(tokens[inner].as_str());
                                        i = close_p + 1;
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }

                // Handle legacy imports
                if id_str == "import" {
                    // Check next non-trivia
                    if let Some(first_tok) = next_non_trivia(i + 1) {
                        let mut line_end = first_tok;
                        while line_end < len && tokens[line_end].as_str() != ";" {
                            line_end += 1;
                        }

                        let import_line: String = tokens[i..=line_end]
                            .iter()
                            .map(|t| t.as_str())
                            .collect();

                        // E.g. `import .foo;` or `import .foo as bar;`
                        if import_line.contains("py_") {
                            out.push_str(&format!("// {import_line}"));
                            i = line_end + 1;
                            continue;
                        } else if import_line.contains('.') {
                            // Extract relative import
                            let trimmed = import_line
                                .trim_start_matches("import")
                                .trim_end_matches(';')
                                .trim();
                            if let Some((mod_part, as_part)) = trimmed.split_once(" as ") {
                                let mod_clean = mod_part.trim().trim_start_matches('.');
                                let as_clean = as_part.trim();
                                out.push_str(&format!(
                                    "import * as {as_clean} from \"./{mod_clean}\";"
                                ));
                            } else {
                                let mod_clean = trimmed.trim_start_matches('.');
                                out.push_str(&format!(
                                    "import * as {mod_clean} from \"./{mod_clean}\";"
                                ));
                            }
                            i = line_end + 1;
                            continue;
                        }
                    }
                }

                // Handle static var -> drop static
                if id_str == "static" {
                    if let Some(next_idx) = next_non_trivia(i + 1) {
                        if tokens[next_idx].as_str() == "var" {
                            i = next_idx;
                            continue;
                        }
                    }
                }

                // Handle `var`
                if id_str == "var" {
                    if in_for_header {
                        out.push_str("let");
                        i += 1;
                        continue;
                    }

                    if in_object {
                        // Class field
                        out.push_str("public ");
                        i += 1;
                        // Read identifiers until ';'
                        while i < len && tokens[i].as_str() != ";" {
                            if let Token::Ident(field_name) = &tokens[i] {
                                out.push_str(&format!("{field_name}: Var = var_(0)"));
                            } else if tokens[i].as_str() == "," {
                                out.push_str("; public ");
                            } else {
                                out.push_str(tokens[i].as_str());
                            }
                            i += 1;
                        }
                        continue;
                    }

                    // Statement-level var declaration: `var x = 0;` or `var x, y = 1, 2;`
                    // Collect tokens until `;`
                    let mut semi_idx = i + 1;
                    while semi_idx < len && tokens[semi_idx].as_str() != ";" {
                        semi_idx += 1;
                    }

                    let stmt_tokens: Vec<&Token> = tokens[i + 1..semi_idx]
                        .iter()
                        .filter(|t| !t.is_trivia())
                        .collect();

                    // Check for multiple assignments: `var a, b = 1, 2`
                    if let Some(eq_pos) = stmt_tokens.iter().position(|t| t.as_str() == "=") {
                        let lhs = &stmt_tokens[..eq_pos];
                        let rhs = &stmt_tokens[eq_pos + 1..];

                        let names: Vec<&str> = lhs
                            .split(|t| t.as_str() == ",")
                            .filter_map(|slice| slice.first().map(|t| t.as_str()))
                            .collect();

                        let vals: Vec<String> = rhs
                            .split(|t| t.as_str() == ",")
                            .map(|slice| {
                                slice.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(" ")
                            })
                            .collect();

                        let mut var_stmts = Vec::new();
                        for (idx, name) in names.iter().enumerate() {
                            let clean_name = name.split(':').next().unwrap_or(name).trim();
                            let default_val = "0".to_string();
                            let init_val = vals.get(idx).unwrap_or(&default_val);

                            // Detect EUDArray initialization
                            if init_val.starts_with("EUDArray(") || init_val.starts_with("new EUDArray(") {
                                declared_arrays.insert(clean_name.to_string());
                            }

                            // If init_val is EUDArray, don't wrap in var_()
                            if init_val.starts_with("EUDArray(") {
                                var_stmts.push(format!("const {clean_name} = new {init_val}"));
                            } else if init_val.starts_with("new EUDArray(") {
                                var_stmts.push(format!("const {clean_name} = {init_val}"));
                            } else {
                                var_stmts.push(format!("const {clean_name} = var_({init_val})"));
                            }
                        }

                        out.push_str(&var_stmts.join("; "));
                        out.push(';');
                        i = semi_idx + 1;
                        continue;
                    } else {
                        // Uninitialized `var x;` or `var x: Type;`
                        let names: Vec<&str> = stmt_tokens
                            .split(|t| t.as_str() == ",")
                            .filter_map(|slice| slice.first().map(|t| t.as_str()))
                            .collect();

                        let mut var_stmts = Vec::new();
                        for name in names {
                            let clean_name = name.split(':').next().unwrap_or(name).trim();
                            var_stmts.push(format!("const {clean_name} = var_(0)"));
                        }
                        out.push_str(&var_stmts.join("; "));
                        out.push(';');
                        i = semi_idx + 1;
                        continue;
                    }
                }

                // Handle `const`
                if id_str == "const" {
                    // Check for multiple constants or EUDArray
                    let mut semi_idx = i + 1;
                    while semi_idx < len && tokens[semi_idx].as_str() != ";" {
                        semi_idx += 1;
                    }

                    let stmt_tokens: Vec<&Token> = tokens[i + 1..semi_idx]
                        .iter()
                        .filter(|t| !t.is_trivia())
                        .collect();

                    if let Some(eq_pos) = stmt_tokens.iter().position(|t| t.as_str() == "=") {
                        let lhs = &stmt_tokens[..eq_pos];
                        let rhs = &stmt_tokens[eq_pos + 1..];

                        let names: Vec<&str> = lhs
                            .split(|t| t.as_str() == ",")
                            .filter_map(|slice| slice.first().map(|t| t.as_str()))
                            .collect();

                        let vals: Vec<String> = rhs
                            .split(|t| t.as_str() == ",")
                            .map(|slice| {
                                slice.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(" ")
                            })
                            .collect();

                        if names.len() > 1 && names.len() == vals.len() {
                            let mut const_stmts = Vec::new();
                            for (idx, name) in names.iter().enumerate() {
                                let init_val = &vals[idx];
                                if init_val.starts_with("EUDArray(") {
                                    declared_arrays.insert(name.to_string());
                                    const_stmts.push(format!("const {name} = new {init_val}"));
                                } else {
                                    const_stmts.push(format!("const {name} = {init_val}"));
                                }
                            }
                            out.push_str(&const_stmts.join("; "));
                            out.push(';');
                            i = semi_idx + 1;
                            continue;
                        } else if names.len() == 1 {
                            let name = names[0];
                            let init_val = vals.first().cloned().unwrap_or_default();
                            if init_val.starts_with("EUDArray(") {
                                declared_arrays.insert(name.to_string());
                            }
                        }
                    }
                    out.push_str("const");
                    i += 1;
                    continue;
                }

                // Handle `function`
                if id_str == "function" {
                    // Check if inside object
                    if in_object {
                        out.push_str("public ");
                    } else {
                        // Check function name
                        if let Some(fn_name_idx) = next_non_trivia(i + 1) {
                            let fn_name = tokens[fn_name_idx].as_str();
                            if fn_name == "main" {
                                has_main = true;
                            }
                            out.push_str("export function ");
                            out.push_str(fn_name);
                            i = fn_name_idx + 1;

                            // Handle parameter list
                            if let Some(open_p) = next_non_trivia(i) {
                                if tokens[open_p].as_str() == "(" {
                                    out.push('(');
                                    let mut param_tokens = Vec::new();
                                    let mut p_idx = open_p + 1;
                                    let mut p_depth = 1;
                                    while p_idx < len {
                                        if tokens[p_idx].as_str() == "(" {
                                            p_depth += 1;
                                        } else if tokens[p_idx].as_str() == ")" {
                                            p_depth -= 1;
                                            if p_depth == 0 {
                                                break;
                                            }
                                        }
                                        param_tokens.push(&tokens[p_idx]);
                                        p_idx += 1;
                                    }

                                    let param_non_trivia: Vec<&Token> = param_tokens
                                        .iter()
                                        .filter(|t| !t.is_trivia())
                                        .cloned()
                                        .collect();

                                    let params: Vec<Vec<&Token>> = param_non_trivia
                                        .split(|t| t.as_str() == ",")
                                        .map(|s| s.to_vec())
                                        .filter(|s| !s.is_empty())
                                        .collect();

                                    let mut typed_params = Vec::new();
                                    for p in params {
                                        if p.iter().any(|t| t.as_str() == ":") {
                                            let s: String = p.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(" ");
                                            typed_params.push(s);
                                        } else if let Some(first) = p.first() {
                                            typed_params.push(format!("{}: Var", first.as_str()));
                                        }
                                    }

                                    out.push_str(&typed_params.join(", "));
                                    out.push(')');
                                    i = p_idx + 1;

                                    // Check if forward declaration ending in `;`
                                    if let Some(next_tok) = next_non_trivia(i) {
                                        if tokens[next_tok].as_str() == ":" {
                                            // Skip return type up to `{` or `;`
                                            let mut r_idx = next_tok + 1;
                                            while r_idx < len && tokens[r_idx].as_str() != "{" && tokens[r_idx].as_str() != ";" {
                                                r_idx += 1;
                                            }
                                            if r_idx < len && tokens[r_idx].as_str() == ";" {
                                                out.push(';');
                                                i = r_idx + 1;
                                                continue;
                                            }
                                            i = r_idx;
                                        }
                                    }
                                    continue;
                                }
                            }
                        }
                    }
                    out.push_str("function");
                    i += 1;
                    continue;
                }

                // Handle `object`
                if id_str == "object" {
                    in_object = true;
                    out.push_str("export class");
                    i += 1;
                    continue;
                }

                // Handle `extends`
                if id_str == "extends" {
                    // If extends EUDStruct, drop it
                    if let Some(next_idx) = next_non_trivia(i + 1) {
                        if tokens[next_idx].as_str() == "EUDStruct" {
                            i = next_idx + 1;
                            continue;
                        }
                    }
                    out.push_str("extends");
                    i += 1;
                    continue;
                }

                // Handle `for`
                if id_str == "for" {
                    in_for_header = true;
                    for_paren_depth = 0;
                    out.push_str("for");
                    i += 1;
                    continue;
                }

                // Handle `foreach`
                if id_str == "foreach" {
                    out.push_str("for");
                    i += 1;
                    continue;
                }

                // Handle `switch` and `epdswitch`
                if id_str == "switch" || id_str == "epdswitch" {
                    out.push_str("switch");
                    if let Some(open_p) = next_non_trivia(i + 1) {
                        if tokens[open_p].as_str() == "(" {
                            // Find matching closing paren
                            let mut p_depth = 1;
                            let mut p_idx = open_p + 1;
                            let mut inner_tokens = Vec::new();
                            while p_idx < len {
                                if tokens[p_idx].as_str() == "(" {
                                    p_depth += 1;
                                } else if tokens[p_idx].as_str() == ")" {
                                    p_depth -= 1;
                                    if p_depth == 0 {
                                        break;
                                    }
                                }
                                inner_tokens.push(&tokens[p_idx]);
                                p_idx += 1;
                            }

                            // If there is a comma, only take expression before comma (omit bitmask)
                            let comma_pos = inner_tokens.iter().position(|t| t.as_str() == ",");
                            let disc_tokens = if let Some(cp) = comma_pos {
                                &inner_tokens[..cp]
                            } else {
                                &inner_tokens[..]
                            };

                            out.push_str(" (");
                            for t in disc_tokens {
                                out.push_str(t.as_str());
                            }
                            out.push(')');
                            i = p_idx + 1;
                            continue;
                        }
                    }
                    i += 1;
                    continue;
                }

                // Handle `once`
                if id_str == "once" {
                    once_counter += 1;
                    let flag_name = format!("_once_flag_{once_counter}");
                    out.push_str(&format!(
                        "{{ const {flag_name} = var_(0); if ({flag_name}.exactly(0)) {{ {flag_name}.setTo(1); "
                    ));

                    // Check if followed by block `{` or statement
                    if let Some(next_idx) = next_non_trivia(i + 1) {
                        if tokens[next_idx].as_str() == "{" {
                            // Find matching `}` and close both blocks
                            i = next_idx + 1;
                            let mut depth = 1;
                            while i < len && depth > 0 {
                                if tokens[i].as_str() == "{" {
                                    depth += 1;
                                    out.push('{');
                                } else if tokens[i].as_str() == "}" {
                                    depth -= 1;
                                    if depth == 0 {
                                        out.push_str("} }");
                                        i += 1;
                                        break;
                                    } else {
                                        out.push('}');
                                    }
                                } else {
                                    out.push_str(tokens[i].as_str());
                                }
                                i += 1;
                            }
                            continue;
                        } else {
                            // Single statement ending with `;`
                            i = next_idx;
                            while i < len && tokens[i].as_str() != ";" {
                                out.push_str(tokens[i].as_str());
                                i += 1;
                            }
                            out.push_str("; } }");
                            if i < len && tokens[i].as_str() == ";" {
                                i += 1;
                            }
                            continue;
                        }
                    }
                    i += 1;
                    continue;
                }

                // Handle `CUnit` instantiation: `CUnit(epd, ptr=ptr)` -> `CUnit.cast(epd, ptr)`
                if id_str == "CUnit" {
                    if let Some(open_p) = next_non_trivia(i + 1) {
                        if tokens[open_p].as_str() == "(" {
                            // Read arguments
                            let mut p_depth = 1;
                            let mut p_idx = open_p + 1;
                            let mut args_raw = Vec::new();
                            while p_idx < len {
                                if tokens[p_idx].as_str() == "(" {
                                    p_depth += 1;
                                } else if tokens[p_idx].as_str() == ")" {
                                    p_depth -= 1;
                                    if p_depth == 0 {
                                        break;
                                    }
                                }
                                args_raw.push(&tokens[p_idx]);
                                p_idx += 1;
                            }

                            let args_str: String = args_raw.iter().map(|t| t.as_str()).collect();
                            // If `ptr = ...` exists, strip `ptr = `
                            let cleaned_args = args_str.replace("ptr=", "").replace("ptr =", "");
                            out.push_str(&format!("CUnit.cast({cleaned_args})"));
                            i = p_idx + 1;
                            continue;
                        }
                    }
                }

                // Handle `u.owner` -> `u.player`
                if id_str == "owner" && i > 0 && tokens[i - 1].as_str() == "." {
                    out.push_str("player");
                    i += 1;
                    continue;
                }

                // Handle memory write functions:
                // dwwrite_epd(epd, val) -> SetMemory(epd, SetTo, val)
                // dwwrite(addr, val) -> SetMemory(addr, SetTo, val)
                if id_str == "dwwrite_epd" || id_str == "dwwrite" {
                    if let Some(open_p) = next_non_trivia(i + 1) {
                        if tokens[open_p].as_str() == "(" {
                            let mut p_depth = 1;
                            let mut p_idx = open_p + 1;
                            let mut args_raw = Vec::new();
                            while p_idx < len {
                                if tokens[p_idx].as_str() == "(" {
                                    p_depth += 1;
                                } else if tokens[p_idx].as_str() == ")" {
                                    p_depth -= 1;
                                    if p_depth == 0 {
                                        break;
                                    }
                                }
                                args_raw.push(&tokens[p_idx]);
                                p_idx += 1;
                            }
                            if let Some(comma_pos) = args_raw.iter().position(|t| t.as_str() == ",") {
                                let epd_str: String = args_raw[..comma_pos].iter().map(|t| t.as_str()).collect();
                                let val_str: String = args_raw[comma_pos + 1..].iter().map(|t| t.as_str()).collect();
                                out.push_str(&format!("SetMemory({}, SetTo, {})", epd_str.trim(), val_str.trim()));
                                i = p_idx + 1;
                                continue;
                            }
                        }
                    }
                }

                // Handle `EUDArray(N)` -> `new EUDArray(N)`
                if id_str == "EUDArray" {
                    if let Some(open_p) = next_non_trivia(i + 1) {
                        if tokens[open_p].as_str() == "(" {
                            out.push_str("new EUDArray");
                            i += 1;
                            continue;
                        }
                    }
                }

                // Array indexing replacement: `arr[idx] = val` -> `arr.set(idx, val)`
                if declared_arrays.contains(id_str) {
                    if let Some(bracket_idx) = next_non_trivia(i + 1) {
                        if tokens[bracket_idx].as_str() == "[" {
                            let mut b_depth = 1;
                            let mut b_idx = bracket_idx + 1;
                            let mut index_tokens = Vec::new();
                            while b_idx < len {
                                if tokens[b_idx].as_str() == "[" {
                                    b_depth += 1;
                                } else if tokens[b_idx].as_str() == "]" {
                                    b_depth -= 1;
                                    if b_depth == 0 {
                                        break;
                                    }
                                }
                                index_tokens.push(&tokens[b_idx]);
                                b_idx += 1;
                            }

                            let index_str: String = index_tokens.iter().map(|t| t.as_str()).collect();

                            if let Some(after_bracket) = next_non_trivia(b_idx + 1) {
                                if tokens[after_bracket].as_str() == "=" {
                                    // Assignment: arr[idx] = val;
                                    let mut semi_pos = after_bracket + 1;
                                    let mut val_tokens = Vec::new();
                                    while semi_pos < len && tokens[semi_pos].as_str() != ";" {
                                        val_tokens.push(&tokens[semi_pos]);
                                        semi_pos += 1;
                                    }
                                    let val_str: String = val_tokens.iter().map(|t| t.as_str()).collect();
                                    out.push_str(&format!("{id_str}.set({index_str}, {val_str})"));
                                    i = semi_pos;
                                    continue;
                                } else {
                                    // Getter: arr[idx]
                                    out.push_str(&format!("{id_str}.get({index_str})"));
                                    i = b_idx + 1;
                                    continue;
                                }
                            }
                        }
                    }
                }

                out.push_str(id_str);
                i += 1;
            }

            Token::Symbol(sym) => {
                let s = sym.as_str();

                // Track parentheses in for loop header
                if in_for_header {
                    if s == "(" {
                        for_paren_depth += 1;
                    } else if s == ")" {
                        for_paren_depth -= 1;
                        if for_paren_depth == 0 {
                            in_for_header = false;
                        }
                    }
                }

                // Track object body end
                if in_object && s == "}" {
                    in_object = false;
                }

                out.push_str(s);
                i += 1;
            }

            Token::Number(num) => {
                out.push_str(num);
                i += 1;
            }

            Token::StringLit(slit) => {
                out.push_str(slit);
                i += 1;
            }
        }
    }

    // Add standard header
    let header = r#"import {
  var_,
  Var,
  CUnit,
  CSprite,
  EUDArray,
  MapString,
  Deaths,
  DeathsX,
  SetDeaths,
  SetDeathsX,
  Memory,
  MemoryX,
  SetMemory,
  SetMemoryX,
  SetResources,
  DisplayText,
  DisplayTextId,
  CreateUnit,
  Comment,
  PreserveTrigger,
  Victory,
  Defeat,
  DoActions,
  Player1,
  Player2,
  Player3,
  Player4,
  Player5,
  Player6,
  Player7,
  Player8,
  CurrentPlayer,
  AllPlayers,
  Ore,
  Gas,
  OreAndGas,
  SetTo,
  Add,
  Subtract,
  AtLeast,
  AtMost,
  Exactly,
  Always,
  Never,
  EPD,
} from "@eudkit/sdk";

"#;

    let mut result = String::with_capacity(header.len() + out.len() + 100);
    result.push_str(header);
    result.push_str(&out);

    // If no main function was exported, add a default main loop
    if !has_main {
        result.push_str("\n\nexport function main() {\n  PreserveTrigger();\n}\n");
    }

    Ok(result)
}
