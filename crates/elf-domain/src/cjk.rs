pub fn contains_cjk(input: &str) -> bool {
    input.chars().any(|c| {
        let code = c as u32;
        matches!(
            code,
            0x3000..=0x303F
                | 0x3040..=0x309F
                | 0x30A0..=0x30FF
                | 0x4E00..=0x9FFF
                | 0xAC00..=0xD7AF
        )
    })
}
