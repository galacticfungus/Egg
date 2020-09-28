use crate::Alphabet;

impl Alphabet {
    pub fn get_range(&self) -> rand::distributions::Uniform<u32> {
        match self {
            Self::Arabic => rand::distributions::Uniform::new(0x0600, 0x06FF),
            Self::Chinese => rand::distributions::Uniform::new(0x4e00, 0x62ff),
            Self::Cyrillic => rand::distributions::Uniform::new(0x0000, 0x04ff),
            Self::Latin => rand::distributions::Uniform::new(0x0041, 0x007f),
            _ => panic!("An unknown alphabet was selected"),
        }
        // let arabic_letters = rand::distributions::Uniform::new(0x0600, 0x06FF);
        // let chinese_symbols = rand::distributions::Uniform::new(0x4e00, 0x62ff);
        // let english_alphabet = rand::distributions::Uniform::new(0x0000, 0x007f);
        // let cyrillic_alphabet = rand::distributions::Uniform::new(0x0400, 0x04ff);
    }

    pub fn get_random_line(&self, rng: &mut impl rand::Rng) -> String {
        let characters_to_use = self.get_range();
        let number_of_words = rng.gen_range(5, 13);
        let mut line_to_write = String::new();
        for word_count in 0..number_of_words {
            if word_count != 0 {
                line_to_write.push(' ');
            }
            let letter_count: usize = rng.gen_range(2, 7);
            let word: String = rng
                .sample_iter(&characters_to_use)
                .map(|index| unsafe { std::char::from_u32_unchecked(index) })
                .filter(|&character| character != '\n' || character != '\r')
                .take(letter_count)
                .collect();
            line_to_write.push_str(word.as_str());
        }
        // line_to_write.push('\n');
        line_to_write
    }
}
