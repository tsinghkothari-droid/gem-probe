use regex::Regex;
use std::sync::LazyLock;

static PLASTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)cement plaster|plastering|internal plaster|external plaster|ceiling plaster|dismantling old plaster|plaster of mix|plaster in cement|mm cement plaster|wall putty").unwrap()
});
static MASONRY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)brick masonry|half brick masonry|brick work with").unwrap()
});
static HOUSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)quarter|dwelling|barrack|hostel|bungalow|guest house|residential building|staff housing|type\s?-?\s?(ii|iii|iv)\b").unwrap()
});
static CIVIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)construct|civil work|masonry|rcc|brick work|plaster").unwrap()
});
static DROP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)plaster of paris bandage|plaster cutting|pop bandage|carton box|ventilator|tracksuit|manpower outsourcing|security manpower").unwrap()
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Plaster,
    Masonry,
    House,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    PlasterLine,
    MasonryLine,
    BuildingWorks,
}

impl Why {
    pub fn label(self) -> &'static str {
        match self {
            Why::PlasterLine => "plaster line",
            Why::MasonryLine => "masonry line",
            Why::BuildingWorks => "building works",
        }
    }
}

pub fn classify(kind: Kind, text: &str) -> Option<Why> {
    if DROP.is_match(text) {
        return None;
    }
    let plaster = PLASTER.is_match(text);
    let masonry = MASONRY.is_match(text);
    match kind {
        Kind::Plaster if plaster => Some(Why::PlasterLine),
        Kind::Masonry if masonry || plaster => {
            if plaster {
                Some(Why::PlasterLine)
            } else {
                Some(Why::MasonryLine)
            }
        }
        Kind::House if HOUSE.is_match(text) && CIVIL.is_match(text) => {
            if plaster {
                Some(Why::PlasterLine)
            } else if masonry {
                Some(Why::MasonryLine)
            } else {
                Some(Why::BuildingWorks)
            }
        }
        _ => None,
    }
}

pub fn snippet(text: &str) -> String {
    let needle = PLASTER
        .find(text)
        .or_else(|| MASONRY.find(text))
        .or_else(|| HOUSE.find(text));
    let (start, end) = match needle {
        Some(m) => {
            let start = m.start().saturating_sub(50);
            let end = (m.end() + 90).min(text.len());
            (start, end)
        }
        None => (0, text.len().min(180)),
    };
    text[start..end].split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_plaster_line() {
        let text = "Supply and making 15 mm thick cement plaster on inside wall";
        assert_eq!(classify(Kind::Plaster, text), Some(Why::PlasterLine));
    }

    #[test]
    fn drops_medical_plaster() {
        let text = "Plaster of Paris Bandage Conforming to IS 4738";
        assert_eq!(classify(Kind::Plaster, text), None);
    }

    #[test]
    fn keeps_a_barrack_when_the_bill_is_civil() {
        let text = "Construction of one new Trg coy incl Barrack, Cook House";
        assert_eq!(classify(Kind::House, text), Some(Why::BuildingWorks));
    }
}
