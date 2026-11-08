//! Version comparison for the catalog's three `version_scheme`s.
//!
//! - `dotted`: the leading run of dot-separated numbers, compared numerically;
//!   anything after it is ignored (snap's `144.0.2-1` reads as `144.0.2`).
//! - `deb`: dpkg's rules, via the `debversion` crate.
//! - `rpm`: rpm's `rpmvercmp` on epoch, version and release. No maintained
//!   crate supports our MSRV, so this follows rpm's `rpmvio.c` directly and is
//!   tested against rpm's own test vectors.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    Dotted,
    Deb,
    Rpm,
}

/// Compares two versions. `None` when either can't be read in this scheme,
/// so callers can skip an entry rather than guess.
pub fn compare(scheme: Scheme, a: &str, b: &str) -> Option<Ordering> {
    match scheme {
        Scheme::Dotted => Some(dotted(a)?.cmp(&dotted(b)?)),
        Scheme::Deb => {
            let a: debversion::Version = a.trim().parse().ok()?;
            let b: debversion::Version = b.trim().parse().ok()?;
            Some(a.cmp(&b))
        }
        Scheme::Rpm => Some(rpm_evr(a.trim(), b.trim())),
    }
}

/// Leading dot-separated numbers, with trailing zeros dropped so `1.0` == `1`.
fn dotted(v: &str) -> Option<Vec<u64>> {
    let mut parts = Vec::new();
    for seg in v.trim().split('.') {
        let digits = seg.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            break;
        }
        parts.push(seg[..digits].parse().ok()?);
        if digits < seg.len() {
            break; // "2-1" or "0rc1": the numeric run ends inside this segment
        }
    }
    if parts.is_empty() {
        return None;
    }
    while parts.len() > 1 && parts.last() == Some(&0) {
        parts.pop();
    }
    Some(parts)
}

/// `[epoch:]version[-release]`. A missing epoch is 0; when either side has no
/// release, releases are not compared (as rpm does for `foo >= 1.0`).
fn rpm_evr(a: &str, b: &str) -> Ordering {
    fn split(v: &str) -> (u64, &str, Option<&str>) {
        let (epoch, rest) = match v.split_once(':') {
            Some((e, r)) if e.bytes().all(|c| c.is_ascii_digit()) => (e.parse().unwrap_or(0), r),
            _ => (0, v),
        };
        match rest.rsplit_once('-') {
            Some((ver, rel)) => (epoch, ver, Some(rel)),
            None => (epoch, rest, None),
        }
    }
    let (ea, va, ra) = split(a);
    let (eb, vb, rb) = split(b);
    ea.cmp(&eb)
        .then_with(|| rpmvercmp(va, vb))
        .then_with(|| match (ra, rb) {
            (Some(ra), Some(rb)) => rpmvercmp(ra, rb),
            _ => Ordering::Equal,
        })
}

/// rpm's segment comparison, including `~` (sorts before anything) and `^`
/// (sorts after the base version but before the next one).
fn rpmvercmp(a: &str, b: &str) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    let (mut one, mut two) = (a.as_bytes(), b.as_bytes());
    let skip = |s: &[u8]| -> usize {
        s.iter()
            .take_while(|&&c| !c.is_ascii_alphanumeric() && c != b'~' && c != b'^')
            .count()
    };
    loop {
        one = &one[skip(one)..];
        two = &two[skip(two)..];

        let (c1, c2) = (one.first().copied(), two.first().copied());
        if c1 == Some(b'~') || c2 == Some(b'~') {
            if c1 != Some(b'~') {
                return Ordering::Greater;
            }
            if c2 != Some(b'~') {
                return Ordering::Less;
            }
            one = &one[1..];
            two = &two[1..];
            continue;
        }
        if c1 == Some(b'^') || c2 == Some(b'^') {
            if c1.is_none() {
                return Ordering::Less;
            }
            if c2.is_none() {
                return Ordering::Greater;
            }
            if c1 != Some(b'^') {
                return Ordering::Greater;
            }
            if c2 != Some(b'^') {
                return Ordering::Less;
            }
            one = &one[1..];
            two = &two[1..];
            continue;
        }
        let (Some(c1), Some(_)) = (c1, c2) else { break };

        let numeric = c1.is_ascii_digit();
        let class = |c: &u8| {
            if numeric {
                c.is_ascii_digit()
            } else {
                c.is_ascii_alphabetic()
            }
        };
        let n1 = one.iter().take_while(|c| class(c)).count();
        let n2 = two.iter().take_while(|c| class(c)).count();
        if n2 == 0 {
            // Different kinds of segment: numbers beat letters.
            return if numeric {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }
        let (s1, s2) = (&one[..n1], &two[..n2]);
        let ord = if numeric {
            let t1 = &s1[s1.iter().take_while(|&&c| c == b'0').count()..];
            let t2 = &s2[s2.iter().take_while(|&&c| c == b'0').count()..];
            t1.len().cmp(&t2.len()).then_with(|| t1.cmp(t2))
        } else {
            s1.cmp(s2)
        };
        if ord != Ordering::Equal {
            return ord;
        }
        one = &one[n1..];
        two = &two[n2..];
    }
    match (one.is_empty(), two.is_empty()) {
        (true, true) => Ordering::Equal,
        (false, _) => Ordering::Greater,
        _ => Ordering::Less,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ordering::*;

    #[test]
    fn dotted_compares_numbers_and_ignores_suffixes() {
        let c = |a, b| compare(Scheme::Dotted, a, b);
        assert_eq!(c("142.0.7444.59", "142.0.7444.162"), Some(Less));
        assert_eq!(c("157.0.8103.39", "157.0.8103.39"), Some(Equal));
        assert_eq!(c("144.0.2-1", "144.0.2"), Some(Equal));
        assert_eq!(c("1.0", "1"), Some(Equal));
        assert_eq!(c("10", "9.9.9"), Some(Greater));
        assert_eq!(c("24.0rc1", "24.0"), Some(Equal));
        assert_eq!(c("v1", "1"), None);
        assert_eq!(c("", "1"), None);
    }

    #[test]
    fn deb_follows_dpkg() {
        let c = |a, b| compare(Scheme::Deb, a, b);
        assert_eq!(c("3.0.13-0ubuntu3.4", "3.0.13-0ubuntu3.5"), Some(Less));
        assert_eq!(c("1:1.0", "2.0"), Some(Greater));
        assert_eq!(c("1.0~rc1", "1.0"), Some(Less));
        assert_eq!(c("3.0.21-1", "3.0.21-1~deb12u1"), Some(Greater));
    }

    // From rpm's tests/rpmvercmp.at.
    #[test]
    fn rpm_matches_rpmvercmp_vectors() {
        let cases: &[(&str, &str, Ordering)] = &[
            ("1.0", "1.0", Equal),
            ("1.0", "2.0", Less),
            ("2.0.1", "2.0.1", Equal),
            ("2.0", "2.0.1", Less),
            ("2.0.1a", "2.0.1", Greater),
            ("5.5p1", "5.5p2", Less),
            ("5.5p10", "5.5p1", Greater),
            ("10xyz", "10.1xyz", Less),
            ("xyz10", "xyz10.1", Less),
            ("xyz.4", "8", Less),
            ("8", "xyz.4", Greater),
            ("5.5p1", "5.5.p1", Equal),
            ("10.0001", "10.1", Equal),
            ("10.0001", "10.0039", Less),
            ("4.999.9", "5.0", Less),
            ("20101121", "20101122", Less),
            ("2_0", "2_0", Equal),
            ("2.0", "2_0", Equal),
            ("a", "a", Equal),
            ("a+", "a_", Equal),
            ("+a", "_a", Equal),
            ("+", "_", Equal),
            ("1.0~rc1", "1.0", Less),
            ("1.0~rc1", "1.0~rc2", Less),
            ("1.0~rc1~git123", "1.0~rc1", Less),
            ("1.0^", "1.0", Greater),
            ("1.0^git1", "1.0^git2", Less),
            ("1.0^git1", "1.01", Less),
            ("1.0^20160101", "1.0.1", Less),
            ("1.0~rc1^git1", "1.0~rc1", Greater),
            ("1.0^git1~pre", "1.0^git1", Less),
        ];
        for &(a, b, want) in cases {
            assert_eq!(rpmvercmp(a, b), want, "{a} vs {b}");
            assert_eq!(rpmvercmp(b, a), want.reverse(), "{b} vs {a}");
        }
    }

    #[test]
    fn rpm_compares_epoch_then_version_then_release() {
        let c = |a, b| compare(Scheme::Rpm, a, b);
        assert_eq!(c("1:3.0.7-27.el9", "3.2.0-1.fc41"), Some(Greater));
        assert_eq!(c("3.2.0-1.fc41", "3.2.0-2.fc41"), Some(Less));
        assert_eq!(c("3.2.0", "3.2.0-2.fc41"), Some(Equal));
    }
}
