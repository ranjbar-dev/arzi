//! Step 6.5 (docs/phase-6-reporting.md §6.5): amount-in-Persian-words, a
//! proper function over a real signed integer — the fix for the legacy
//! `Dm.Str2String`'s (`Dmu.pas:604-635`) fragility (04-06-a.md §6.4):
//! "It takes a **string**, not an integer, and slices it with `Copy` — a
//! negative sign or a thousands separator in the input produces garbage."
//! Since every call site here is `money::round_to_rial`'s own `i64` output
//! (or a plain `i64` column), a pre-formatted string never reaches this
//! function in the first place — the fragility class cannot recur.
//!
//! **Spelling, a deliberate deviation from the legacy's exact bytes**:
//! 04-06-a.md §6.4 flags "The scale words are spelled with Arabic yeh `ي`
//! ... Preserve the spelling if byte-identical output matters; otherwise
//! normalise." Nothing in this system needs byte-identical output against
//! the legacy (this is a fresh rebuild, not a migration echoing old
//! documents) — normalised to Persian yeh `ی`, matching every other Persian
//! string already in this codebase (`lib/i18n/fa.ts` etc.). `تریلیارد`
//! (§6.4's non-standard 10¹² term) is also not reproduced; the standard
//! `تریلیون` is used instead.
//!
//! **Scale ceiling, a documented boundary, not an oversight**: scale words
//! are defined up to `تریلیون` (10¹²) — any realistic rial amount in this
//! domain's accounting is well under that. An amount at or beyond 10¹⁵
//! falls back to a plain grouped-digit string rather than fabricating a
//! non-standard higher scale word; `render` documents this explicitly.

const ONES: [&str; 10] = [
    "", "یک", "دو", "سه", "چهار", "پنج", "شش", "هفت", "هشت", "نه",
];
const TEENS: [&str; 10] = [
    "ده",
    "یازده",
    "دوازده",
    "سیزده",
    "چهارده",
    "پانزده",
    "شانزده",
    "هفده",
    "هجده",
    "نوزده",
];
const TENS: [&str; 10] = [
    "",
    "",
    "بیست",
    "سی",
    "چهل",
    "پنجاه",
    "شصت",
    "هفتاد",
    "هشتاد",
    "نود",
];
const HUNDREDS: [&str; 10] = [
    "",
    "صد",
    "دویست",
    "سیصد",
    "چهارصد",
    "پانصد",
    "ششصد",
    "هفتصد",
    "هشتصد",
    "نهصد",
];
/// Index 0 = no scale word (the lowest 3-digit group), index 1 = هزار
/// (10³) ... index 4 = تریلیون (10¹²). See the module doc comment's "scale
/// ceiling" note for what happens past this.
const SCALES: [&str; 5] = ["", "هزار", "میلیون", "میلیارد", "تریلیون"];

/// One 0-999 group, e.g. 507 -> "پانصد و هفت".
fn group_to_words(n: u32) -> String {
    debug_assert!(n < 1000);
    let hundreds_digit = n / 100;
    let remainder = n % 100;
    let mut parts = Vec::new();
    if hundreds_digit > 0 {
        parts.push(HUNDREDS[hundreds_digit as usize].to_string());
    }
    if remainder > 0 {
        if remainder < 10 {
            parts.push(ONES[remainder as usize].to_string());
        } else if remainder < 20 {
            parts.push(TEENS[(remainder - 10) as usize].to_string());
        } else {
            let tens_digit = remainder / 10;
            let ones_digit = remainder % 10;
            let mut tens_part = TENS[tens_digit as usize].to_string();
            if ones_digit > 0 {
                tens_part.push_str(" و ");
                tens_part.push_str(ONES[ones_digit as usize]);
            }
            parts.push(tens_part);
        }
    }
    parts.join(" و ")
}

/// Renders a rial amount (money is `i64` rials end-to-end, never a
/// formatted string) as Persian words, e.g. `1_234_567` ->
/// "یک میلیون و دویست و سی و چهار هزار و پانصد و شصت و هفت". Zero ->
/// "صفر". Negative amounts get a leading "منفی" ("negative") — the legacy
/// has no negative convention at all in print (04-06-b.md §6.8: "there is no
/// parenthesis convention and no minus-sign convention anywhere in the
/// printed output"), but nothing in this step's spec asks for negative
/// totals to be unrepresentable, so the ambiguity is resolved with an
/// explicit word rather than silently misreporting the sign.
pub fn amount_in_words(amount: i64) -> String {
    if amount == 0 {
        return "صفر".to_string();
    }
    let negative = amount < 0;
    // i64::MIN has no positive counterpart -- widen to u64 via unsigned_abs
    // rather than risk an overflow panic on the rare edge case.
    let mut magnitude = amount.unsigned_abs();

    if magnitude >= 1_000_000_000_000_000 {
        // Past the documented scale ceiling (see module doc comment) --
        // a plain grouped-digit fallback, never a fabricated scale word.
        let sign = if negative { "-" } else { "" };
        return format!("{sign}{}", group_digits(magnitude));
    }

    let mut groups = Vec::new();
    let mut scale = 0;
    while magnitude > 0 {
        let group = (magnitude % 1000) as u32;
        if group > 0 {
            let mut phrase = group_to_words(group);
            if scale > 0 {
                phrase.push(' ');
                phrase.push_str(SCALES[scale]);
            }
            groups.push(phrase);
        }
        magnitude /= 1000;
        scale += 1;
    }
    groups.reverse();
    let words = groups.join(" و ");
    if negative {
        format!("منفی {words}")
    } else {
        words
    }
}

/// Plain thousands-separated fallback for the scale-ceiling case above --
/// `#,###` shape, ASCII digits (this codebase's own convention, per
/// `04-02-a.md`'s note that the *data* stays ASCII digits regardless of
/// font/locale display preference, 04-06-b.md §6.7).
fn group_digits(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut groups = Vec::new();
    while n > 0 {
        groups.push(format!("{:03}", n % 1000));
        n /= 1000;
    }
    groups.reverse();
    let mut s = groups.join(",");
    // Strip leading zero-padding on the first group.
    while s.starts_with('0') && !s.starts_with("0,") {
        s.remove(0);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_and_simple_cases() {
        assert_eq!(amount_in_words(0), "صفر");
        assert_eq!(amount_in_words(1), "یک");
        assert_eq!(amount_in_words(19), "نوزده");
        assert_eq!(amount_in_words(21), "بیست و یک");
        assert_eq!(amount_in_words(100), "صد");
        assert_eq!(amount_in_words(507), "پانصد و هفت");
    }

    #[test]
    fn multi_group_amount() {
        // 1,234,567 -> one million, two hundred thirty-four thousand,
        // five hundred sixty-seven.
        assert_eq!(
            amount_in_words(1_234_567),
            "یک میلیون و دویست و سی و چهار هزار و پانصد و شصت و هفت"
        );
    }

    #[test]
    fn a_group_that_is_entirely_zero_is_skipped() {
        // 1,000,001 -- the thousands group is zero and must not print
        // "هزار" on its own with nothing in it.
        assert_eq!(amount_in_words(1_000_001), "یک میلیون و یک");
    }

    #[test]
    fn negative_amount_gets_an_explicit_sign_word() {
        assert_eq!(amount_in_words(-500), "منفی پانصد");
    }

    #[test]
    fn round_number_worked_example() {
        // A plausible pistachio-purchase total (05-08-a.md Example A's own
        // 2,346,250,000) -- exercises the میلیارد (billion) scale.
        assert_eq!(
            amount_in_words(2_346_250_000),
            "دو میلیارد و سیصد و چهل و شش میلیون و دویست و پنجاه هزار"
        );
    }
}
