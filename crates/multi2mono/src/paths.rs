use std::path::PathBuf;

pub fn trim_quotes(s: &str) -> &str {
    s.trim().trim_matches('"')
}

/// 解析输入中的路径，支持引号包裹的含空格路径。
pub fn parse_paths(input: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut rest = input.trim();

    while !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix('"') {
            match stripped.find('"') {
                Some(end) => {
                    paths.push(PathBuf::from(&stripped[..end]));
                    rest = stripped[end + 1..].trim_start();
                }
                None => break,
            }
        } else {
            match rest.find(' ') {
                Some(pos) => {
                    let token = rest[..pos].trim();
                    if !token.is_empty() {
                        paths.push(PathBuf::from(trim_quotes(token)));
                    }
                    rest = rest[pos..].trim_start();
                }
                None => {
                    let token = trim_quotes(rest);
                    if !token.is_empty() {
                        paths.push(PathBuf::from(token));
                    }
                    break;
                }
            }
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_and_plain_paths() {
        let input = r#""D:\a b\one.wav" D:\two.wav"#;
        let paths = parse_paths(input);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from(r"D:\a b\one.wav"));
        assert_eq!(paths[1], PathBuf::from(r"D:\two.wav"));
    }
}
