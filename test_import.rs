mod a {
    use std::path::Path;

    #[cfg(test)]
    mod b {
        use super::*;

        #[test]
        fn test_path() {
            let _ = Path::new(".");
        }
    }
}
