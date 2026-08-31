use std::collections::BTreeMap;

/// The nearest-rank median of a sorted integer sample, which stays an integer
/// because it selects a member rather than averaging two.
///
/// One implementation serves every caller that needs a median, so a package's
/// fan-in median and a directory's change amplification are the same statistic
/// stated about different samples.
pub(crate) fn nearest_rank_median(sorted: &[u32]) -> u32 {
    match sorted.len() {
        0 => 0,
        length => sorted[nearest_rank_position(length)],
    }
}

/// The nearest-rank median of a sample counted rather than listed, where the
/// key is a sample value and its count is how many members hold it.
///
/// A change-amplification histogram is a counted sample: it holds at most one
/// key per distinct file count, so the median is selected by walking the counts
/// rather than by expanding the sample a commit at a time. Both forms select
/// the same member of the same sorted sample, because both ask
/// [`nearest_rank_position`] for it.
pub(crate) fn nearest_rank_median_of_counts(counts: &BTreeMap<u32, u32>) -> u32 {
    let length: usize = counts.values().map(|&count| count as usize).sum();
    if length == 0 {
        return 0;
    }
    let position = nearest_rank_position(length);
    let mut passed = 0;
    counts
        .iter()
        .find_map(|(&value, &count)| {
            passed += count as usize;
            (position < passed).then_some(value)
        })
        .unwrap_or(0)
}

/// The zero-based position the nearest-rank median selects in a sorted sample
/// of `length` members, which is the one rule both forms share.
///
/// An even sample takes its lower middle member, which is what keeps the
/// statistic an integer of the sample rather than an average between two.
const fn nearest_rank_position(length: usize) -> usize {
    (length - 1) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_even_sample_takes_the_lower_middle_member_and_an_empty_sample_is_zero() {
        assert_eq!(nearest_rank_median(&[]), 0);
        assert_eq!(nearest_rank_median(&[4]), 4);
        assert_eq!(nearest_rank_median(&[2, 3, 3, 4, 9]), 3);
        assert_eq!(nearest_rank_median(&[1, 3, 5, 9]), 3);
    }

    #[test]
    fn a_counted_sample_selects_the_member_the_listed_sample_selects() {
        assert_eq!(nearest_rank_median_of_counts(&BTreeMap::new()), 0);
        // The accepted example: the observations 2, 3, 3, 4 and 9 counted.
        let counted = BTreeMap::from([(2, 1), (3, 2), (4, 1), (9, 1)]);
        assert_eq!(nearest_rank_median_of_counts(&counted), 3);
    }

    /// The two forms are one statistic, so every sample either form can hold
    /// gives one answer. The samples below cover a single member, both
    /// parities, a run that spans the middle, and a lopsided tail.
    #[test]
    fn both_forms_answer_the_same_for_the_same_sample() {
        let samples: [&[u32]; 7] = [
            &[4],
            &[1, 2],
            &[2, 3, 3, 4, 9],
            &[1, 1, 1, 1, 5, 5],
            &[7, 7, 7],
            &[1, 2, 2, 2, 2, 2, 900],
            &[1, 1_000, 1_000, 1_000],
        ];
        for sample in samples {
            let mut counted: BTreeMap<u32, u32> = BTreeMap::new();
            for &value in sample {
                *counted.entry(value).or_default() += 1;
            }
            assert_eq!(
                nearest_rank_median_of_counts(&counted),
                nearest_rank_median(sample),
                "{sample:?}"
            );
        }
    }
}
