use crate::Instability;
pub const fn instability(incoming: u32, outgoing: u32) -> Option<Instability> {
    Instability::new(outgoing, incoming + outgoing)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn represents_exact_fraction() {
        let value = instability(3, 1).unwrap();
        assert_eq!((value.numerator(), value.denominator()), (1, 4));
        assert_eq!(instability(0, 0), None);
    }
}
