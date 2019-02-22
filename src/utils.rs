use relative_path::{RelativePath, RelativePathBuf};
use std::borrow::Cow;

/// Convert a number into a uppercase radix.
pub fn as_uppercase_radix(mut index: usize) -> String {
    const BASE: u32 = 'A' as u32;
    const DIV: usize = ('Z' as u32 - BASE) as usize + 1;

    let mut buf = Vec::new();
    let mut count = 0usize;

    while index > 0 {
        buf.extend(std::char::from_u32(BASE + (index % DIV) as u32));
        index = index / DIV;
        count += 1;
    }

    buf.extend(std::iter::repeat('A').take(2usize.saturating_sub(count)));
    buf.into_iter().rev().collect::<String>()
}

/// Apply a file prefix to a path.
pub fn path_file_prefix<'a>(
    prefix: Option<&str>,
    path: Cow<'a, RelativePath>,
) -> Cow<'a, RelativePath> {
    let prefix = match prefix {
        Some(prefix) => prefix,
        None => return path,
    };

    let name = match path.file_name() {
        Some(existing) => format!("{}{}", prefix, existing),
        None => prefix.to_string(),
    };

    Cow::Owned(path.with_file_name(name))
}

/// Apply a file suffix to a path.
pub fn path_file_suffix<'a>(
    suffix: Option<&str>,
    path: Cow<'a, RelativePath>,
) -> Cow<'a, RelativePath> {
    let suffix = match suffix {
        Some(suffix) => suffix,
        None => return path,
    };

    let name = match path.file_name() {
        Some(existing) => format!("{}{}", existing, suffix),
        None => suffix.to_string(),
    };

    Cow::Owned(path.with_file_name(name))
}

/// Handle path enumeration.
/// This replaces the first occurence of `$` with as many numbers as needed.
pub fn path_enumeration<'a>(index: usize, path: Cow<'a, RelativePath>) -> Cow<'a, RelativePath> {
    let s = path.as_str();

    let prefix_i = match s.find("$") {
        None => return path,
        Some(prefix_i) => prefix_i,
    };

    let mut buffer = String::with_capacity(s.len());
    let (prefix, rest) = s.split_at(prefix_i);

    if rest.starts_with("$@") {
        buffer.push_str(prefix);
        buffer.push_str(&as_uppercase_radix(index));
        buffer.push_str(&rest[2..]);
        return Cow::Owned(RelativePathBuf::from(buffer));
    }

    let rest_i;
    let mut width = 0;
    let mut it = rest.char_indices();

    loop {
        rest_i = match it.next() {
            Some((_, '$')) => {
                width += 1;
                continue;
            }
            Some((n, _)) => n,
            None => rest.len(),
        };

        break;
    }

    buffer.push_str(prefix);
    buffer.push_str(&format!("{:0width$}", index + 1, width = width));

    if rest_i < rest.len() {
        buffer.push_str(&rest[rest_i..]);
    }

    Cow::Owned(RelativePathBuf::from(buffer))
}

#[cfg(test)]
mod tests {
    use super::{as_uppercase_radix, path_enumeration};
    use relative_path::RelativePath;
    use std::borrow::Cow;

    #[test]
    fn test_path_enumeration() {
        let path = Cow::Borrowed(RelativePath::new("foo/bar/$"));
        let path = path_enumeration(0, path);
        assert_eq!("foo/bar/1", path.as_str());

        let path = Cow::Borrowed(RelativePath::new("foo/bar$$$/foo"));
        let path = path_enumeration(2, path);
        assert_eq!("foo/bar003/foo", path.as_str());

        let path = Cow::Borrowed(RelativePath::new("foo/bar$@/foo"));
        let path = path_enumeration(0, path);
        assert_eq!("foo/barAA/foo", path.as_str());
    }

    #[test]
    fn test_uppercase_radix() {
        assert_eq!("AA", as_uppercase_radix(0));
        assert_eq!("AB", as_uppercase_radix(1));
        assert_eq!("AZ", as_uppercase_radix(25));
        assert_eq!("BA", as_uppercase_radix(26));
        assert_eq!("BB", as_uppercase_radix(27));
        assert_eq!("BZ", as_uppercase_radix(51));
        assert_eq!("CA", as_uppercase_radix(52));
    }
}
