use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

pub const CASE_ERROR: &str = "case must be one of: 'snake', 'camel', 'pascal', 'constant'";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaseStyle {
    Snake,
    Camel,
    Pascal,
    Constant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameError {
    message: String,
}

impl NameError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for NameError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for NameError {}

pub fn make_clean_names(names: &[String], case: &str) -> Result<Vec<String>, NameError> {
    let case = parse_case(case)?;
    let bases = names
        .iter()
        .map(|name| clean_one(name, case))
        .collect::<Vec<_>>();
    Ok(dedupe(&bases, case))
}

fn parse_case(case: &str) -> Result<CaseStyle, NameError> {
    match case {
        "snake" => Ok(CaseStyle::Snake),
        "camel" => Ok(CaseStyle::Camel),
        "pascal" => Ok(CaseStyle::Pascal),
        "constant" => Ok(CaseStyle::Constant),
        _ => Err(NameError::new(CASE_ERROR)),
    }
}

fn clean_one(name: &str, case: CaseStyle) -> String {
    let tokens = tokens(name);
    let tokens = if tokens.is_empty() {
        vec!["x".to_string()]
    } else {
        tokens
    };
    ensure_identifier_start(&format_tokens(&tokens, case), case)
}

fn tokens(name: &str) -> Vec<String> {
    let translated = translate_common_symbols_and_letters(name.trim());
    let normalized = translated
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .collect::<String>();
    tokenize_ascii(&normalized)
}

fn translate_common_symbols_and_letters(value: &str) -> String {
    let mut translated = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            'ß' => translated.push_str("ss"),
            'ẞ' => translated.push_str("SS"),
            'æ' => translated.push_str("ae"),
            'Æ' => translated.push_str("AE"),
            'œ' => translated.push_str("oe"),
            'Œ' => translated.push_str("OE"),
            'ø' => translated.push('o'),
            'Ø' => translated.push('O'),
            'ð' => translated.push('d'),
            'Ð' => translated.push('D'),
            'þ' => translated.push_str("th"),
            'Þ' => translated.push_str("Th"),
            'ł' => translated.push('l'),
            'Ł' => translated.push('L'),
            '%' => translated.push_str(" percent "),
            '#' => translated.push_str(" number "),
            '&' => translated.push_str(" and "),
            '@' => translated.push_str(" at "),
            '+' => translated.push_str(" plus "),
            _ => translated.push(character),
        }
    }
    translated
}

fn tokenize_ascii(value: &str) -> Vec<String> {
    let chars = value.chars().collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut current = String::new();

    for (index, character) in chars.iter().copied().enumerate() {
        if !character.is_ascii_alphanumeric() {
            push_current(&mut output, &mut current);
            continue;
        }

        if !current.is_empty() {
            let previous = chars[index - 1];
            let next = chars.get(index + 1).copied();
            if is_token_boundary(previous, character, next) {
                push_current(&mut output, &mut current);
            }
        }

        current.push(character.to_ascii_lowercase());
    }

    push_current(&mut output, &mut current);
    output
}

fn is_token_boundary(previous: char, current: char, next: Option<char>) -> bool {
    if previous.is_ascii_uppercase()
        && current.is_ascii_uppercase()
        && next.is_some_and(|next_character| next_character.is_ascii_lowercase())
    {
        return true;
    }
    if (previous.is_ascii_lowercase() || previous.is_ascii_digit()) && current.is_ascii_uppercase()
    {
        return true;
    }
    (previous.is_ascii_alphabetic() && current.is_ascii_digit())
        || (previous.is_ascii_digit() && current.is_ascii_alphabetic())
}

fn push_current(output: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        output.push(std::mem::take(current));
    }
}

fn format_tokens(tokens: &[String], case: CaseStyle) -> String {
    match case {
        CaseStyle::Snake => tokens.join("_"),
        CaseStyle::Constant => tokens.join("_").to_uppercase(),
        CaseStyle::Camel => {
            let mut formatted = tokens[0].clone();
            for token in &tokens[1..] {
                formatted.push_str(&capitalize_token(token));
            }
            formatted
        }
        CaseStyle::Pascal => tokens
            .iter()
            .map(|token| capitalize_token(token))
            .collect::<String>(),
    }
}

fn capitalize_token(token: &str) -> String {
    let mut chars = token.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut capitalized = first.to_ascii_uppercase().to_string();
            capitalized.push_str(chars.as_str());
            capitalized
        }
    }
}

fn ensure_identifier_start(name: &str, case: CaseStyle) -> String {
    if !name.starts_with(|character: char| character.is_ascii_digit()) {
        return name.to_string();
    }
    match case {
        CaseStyle::Constant => format!("X_{name}"),
        CaseStyle::Pascal => format!("X{name}"),
        CaseStyle::Camel => format!("x{name}"),
        CaseStyle::Snake => format!("x_{name}"),
    }
}

fn dedupe(names: &[String], case: CaseStyle) -> Vec<String> {
    let mut used = HashSet::new();
    let mut next_suffix_by_base: HashMap<&str, usize> = HashMap::new();
    let mut output = Vec::with_capacity(names.len());

    for name in names {
        let mut candidate = name.clone();
        let mut suffix = next_suffix_by_base.get(name.as_str()).copied().unwrap_or(2);

        while used.contains(&candidate) {
            candidate = with_suffix(name, suffix, case);
            suffix += 1;
        }

        next_suffix_by_base.insert(name, suffix);
        used.insert(candidate.clone());
        output.push(candidate);
    }

    output
}

fn with_suffix(name: &str, suffix: usize, case: CaseStyle) -> String {
    match case {
        CaseStyle::Camel | CaseStyle::Pascal => format!("{name}{suffix}"),
        CaseStyle::Snake | CaseStyle::Constant => format!("{name}_{suffix}"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::*;

    #[test]
    fn cleans_representative_janitor_names() {
        let names = [
            "Customer ID",
            "Customer ID",
            "% Complete",
            "Mötley Crüe",
            "",
            "",
            "OrderID",
            "1st Sale",
        ]
        .map(String::from);

        assert_eq!(
            make_clean_names(&names, "snake").unwrap(),
            vec![
                "customer_id",
                "customer_id_2",
                "percent_complete",
                "motley_crue",
                "x",
                "x_2",
                "order_id",
                "x_1_st_sale"
            ]
        );
    }

    #[test]
    fn supports_all_case_styles() {
        let names = ["Customer ID", "% Complete", "alreadyClean"].map(String::from);

        assert_eq!(
            make_clean_names(&names, "snake").unwrap(),
            vec!["customer_id", "percent_complete", "already_clean"]
        );
        assert_eq!(
            make_clean_names(&names, "constant").unwrap(),
            vec!["CUSTOMER_ID", "PERCENT_COMPLETE", "ALREADY_CLEAN"]
        );
        assert_eq!(
            make_clean_names(&names, "camel").unwrap(),
            vec!["customerId", "percentComplete", "alreadyClean"]
        );
        assert_eq!(
            make_clean_names(&names, "pascal").unwrap(),
            vec!["CustomerId", "PercentComplete", "AlreadyClean"]
        );
    }

    #[test]
    fn duplicate_suffixes_are_stable() {
        let names = ["a", "a", "a_2", "a"].map(String::from);

        assert_eq!(
            make_clean_names(&names, "snake").unwrap(),
            vec!["a", "a_2", "a_2_2", "a_3"]
        );
    }

    #[test]
    fn rejects_invalid_case() {
        assert_eq!(
            make_clean_names(&[String::from("a")], "kebab")
                .unwrap_err()
                .message(),
            CASE_ERROR
        );
    }

    proptest! {
        #[test]
        fn cleaned_names_preserve_length_and_are_unique_nonempty(
            names in proptest::collection::vec(any::<String>(), 0..128)
        ) {
            let cleaned = make_clean_names(&names, "snake").unwrap();
            let unique = cleaned.iter().collect::<HashSet<_>>();

            prop_assert_eq!(cleaned.len(), names.len());
            prop_assert_eq!(unique.len(), cleaned.len());
            prop_assert!(cleaned.iter().all(|name| !name.is_empty()));
        }

        #[test]
        fn cleaned_names_are_idempotent(
            names in proptest::collection::vec(any::<String>(), 0..128)
        ) {
            let cleaned = make_clean_names(&names, "snake").unwrap();

            prop_assert_eq!(make_clean_names(&cleaned, "snake").unwrap(), cleaned);
        }
    }
}
