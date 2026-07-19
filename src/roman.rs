pub(crate) fn convert_single(s: &str) -> Result<i64, String> {
    if s.is_empty() {
        return Err("numerus Romanus vacuus".into());
    }
    fn val(c: char) -> Option<i64> {
        match c {
            'I' => Some(1), 'V' => Some(5), 'X' => Some(10), 'L' => Some(50),
            'C' => Some(100), 'D' => Some(500), 'M' => Some(1000), _ => None,
        }
    }
    let chars: Vec<char> = s.chars().collect();
    let mut total = 0i64;
    for i in 0..chars.len() {
        let cur = val(chars[i]).ok_or_else(|| format!("numerus Romanus invalidus: '{}'", s))?;
        let next = chars.get(i + 1).and_then(|c| val(*c)).unwrap_or(0);
        if cur < next { total -= cur; } else { total += cur; }
    }
    Ok(total)
}

pub(crate) fn convert_int_roman(s: &str) -> Result<i64, String> {
    let nums: Vec<i64> = s
        .split('.')
        .map(convert_single)
        .collect::<Result<_, _>>()?;
    let mut total = 0i64;
    let mut i = 0;
    while i + 1 < nums.len() {
        total += nums[i] * nums[i + 1];
        i += 2;
    }
    if nums.len() % 2 == 1 {
        total += nums[nums.len() - 1];
    }
    Ok(total)
}

#[derive(Debug, Clone, PartialEq)]
pub enum RomanNumber {
    Int(i64),
    Float(f64),
}

pub fn parse_roman(literal: &str) -> Result<RomanNumber, String> {
    if literal == "nullus" {
        return Ok(RomanNumber::Int(0));
    }
    let body = literal
        .strip_prefix('°')
        .ok_or_else(|| format!("numerus sine '°': '{}'", literal))?;

    let mut parts = body.splitn(2, ':');
    let int_str = parts.next().unwrap();
    let int_val = convert_int_roman(int_str)?;

    match parts.next() {
        None => Ok(RomanNumber::Int(int_val)),
        Some(frac) => {
            let mut fp = frac.splitn(2, '/');
            let num_str = fp.next().ok_or("fractio sine numeratore")?;
            let den_str = fp.next().ok_or("fractio sine denominatore")?;
            let num = convert_int_roman(num_str)?;
            let den = convert_int_roman(den_str)?;
            if den == 0 {
                return Err("denominator est nullus".into());
            }
            Ok(RomanNumber::Float(int_val as f64 + num as f64 / den as f64))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_basic_and_subtractive() {
        assert_eq!(convert_single("XXV").unwrap(), 25);
        assert_eq!(convert_single("VII").unwrap(), 7);
        assert_eq!(convert_single("IV").unwrap(), 4);
        assert_eq!(convert_single("IX").unwrap(), 9);
        assert_eq!(convert_single("M").unwrap(), 1000);
    }

    #[test]
    fn single_rejects_garbage() {
        assert!(convert_single("XQ").is_err());
    }

    #[test]
    fn int_multiplicative_pairs() {
        // XXX.M.V.C.VII = 30*1000 + 5*100 + 7 = 30507
        assert_eq!(convert_int_roman("XXX.M.V.C.VII").unwrap(), 30507);
        // VII.M = 7*1000 = 7000  (even number of chunks: pure product)
        assert_eq!(convert_int_roman("VII.M").unwrap(), 7000);
        // single chunk, no dots
        assert_eq!(convert_int_roman("XXV").unwrap(), 25);
    }

    #[test]
    fn parse_integer_literal() {
        assert_eq!(parse_roman("°XXV").unwrap(), RomanNumber::Int(25));
        assert_eq!(parse_roman("°XXX.M.V.C.VII").unwrap(), RomanNumber::Int(30507));
    }

    #[test]
    fn parse_nullus_is_zero() {
        assert_eq!(parse_roman("nullus").unwrap(), RomanNumber::Int(0));
    }

    #[test]
    fn parse_fraction_literal() {
        assert_eq!(parse_roman("°V:I/II").unwrap(), RomanNumber::Float(5.5));
    }

    #[test]
    fn parse_rejects_missing_degree() {
        assert!(parse_roman("XXV").is_err());
    }
}
