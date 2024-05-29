wrap_fixed_bytes!(
    /// Core Blockchain 171 Signature type
    pub struct B1368<171>;
);

mod tests {
    #[test]
    fn test_b1368() {
        let mut b = crate::bits::sig::B1368::default();
        assert_eq!(b.len(), 171);
        b[0] = 1;
        assert_eq!(b[0], 1);
    }
}
