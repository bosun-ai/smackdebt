/// The nearest-rank median of a sorted integer sample, which stays an integer
/// because it selects a member rather than averaging two.
///
/// One implementation serves every caller that needs a median, so a package's
/// fan-in median and a directory's change amplification are the same statistic
/// stated about different samples.
pub(crate) fn nearest_rank_median(sorted: &[u32]) -> u32 {
    match sorted.len() {
        0 => 0,
        length => sorted[(length - 1) / 2],
    }
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
}
