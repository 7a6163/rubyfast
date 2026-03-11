use std::fmt;

use crate::fix::Fix;

/// All 19 performance offense kinds detected by fasterer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OffenseKind {
    ShuffleFirstVsSample,
    SelectFirstVsDetect,
    SelectLastVsReverseDetect,
    ReverseEachVsReverseEach,
    KeysEachVsEachKey,
    MapFlattenVsFlatMap,
    GsubVsTr,
    SortVsSortBy,
    FetchWithArgumentVsBlock,
    HashMergeBangVsHashBrackets,
    BlockVsSymbolToProc,
    EachWithIndexVsWhile,
    IncludeVsCoverOnRange,
    ModuleEval,
    RescueVsRespondTo,
    ProcCallVsYield,
    GetterVsAttrReader,
    SetterVsAttrWriter,
    ForLoopVsEach,
}

impl OffenseKind {
    /// The YAML config key for this offense (matches original fasterer).
    pub fn config_key(self) -> &'static str {
        match self {
            Self::ShuffleFirstVsSample => "shuffle_first_vs_sample",
            Self::SelectFirstVsDetect => "select_first_vs_detect",
            Self::SelectLastVsReverseDetect => "select_last_vs_reverse_detect",
            Self::ReverseEachVsReverseEach => "reverse_each_vs_reverse_each",
            Self::KeysEachVsEachKey => "keys_each_vs_each_key",
            Self::MapFlattenVsFlatMap => "map_flatten_vs_flat_map",
            Self::GsubVsTr => "gsub_vs_tr",
            Self::SortVsSortBy => "sort_vs_sort_by",
            Self::FetchWithArgumentVsBlock => "fetch_with_argument_vs_block",
            Self::HashMergeBangVsHashBrackets => "hash_merge_bang_vs_hash_brackets",
            Self::BlockVsSymbolToProc => "block_vs_symbol_to_proc",
            Self::EachWithIndexVsWhile => "each_with_index_vs_while",
            Self::IncludeVsCoverOnRange => "include_vs_cover_on_range",
            Self::ModuleEval => "module_eval",
            Self::RescueVsRespondTo => "rescue_vs_respond_to",
            Self::ProcCallVsYield => "proc_call_vs_yield",
            Self::GetterVsAttrReader => "getter_vs_attr_reader",
            Self::SetterVsAttrWriter => "setter_vs_attr_writer",
            Self::ForLoopVsEach => "for_loop_vs_each",
        }
    }

    /// Human-readable explanation (matches original fasterer messages).
    pub fn explanation(self) -> &'static str {
        match self {
            Self::ShuffleFirstVsSample => "Array#shuffle.first is slower than Array#sample",
            Self::SelectFirstVsDetect => "Array#select.first is slower than Array#detect",
            Self::SelectLastVsReverseDetect => {
                "Array#select.last is slower than Array#reverse.detect"
            }
            Self::ReverseEachVsReverseEach => {
                "Array#reverse.each is slower than Array#reverse_each"
            }
            Self::KeysEachVsEachKey => {
                "Hash#keys.each is slower than Hash#each_key. \
                 N.B. Hash#each_key cannot be used if the hash is modified during the each block"
            }
            Self::MapFlattenVsFlatMap => "Array#map.flatten(1) is slower than Array#flat_map",
            Self::GsubVsTr => {
                "Using tr is faster than gsub when replacing a single character \
                 in a string with another single character"
            }
            Self::SortVsSortBy => "Enumerable#sort is slower than Enumerable#sort_by",
            Self::FetchWithArgumentVsBlock => {
                "Hash#fetch with second argument is slower than Hash#fetch with block"
            }
            Self::HashMergeBangVsHashBrackets => {
                "Hash#merge! with one argument is slower than Hash#[]"
            }
            Self::BlockVsSymbolToProc => {
                "Calling argumentless methods within blocks is slower than \
                 using symbol to proc"
            }
            Self::EachWithIndexVsWhile => "Using each_with_index is slower than while loop",
            Self::IncludeVsCoverOnRange => "Use #cover? instead of #include? on ranges",
            Self::ModuleEval => "Using module_eval is slower than define_method",
            Self::RescueVsRespondTo => "Don't rescue NoMethodError, rather check with respond_to?",
            Self::ProcCallVsYield => "Calling blocks with call is slower than yielding",
            Self::GetterVsAttrReader => "Use attr_reader for reading ivars",
            Self::SetterVsAttrWriter => "Use attr_writer for writing to ivars",
            Self::ForLoopVsEach => "For loop is slower than using each",
        }
    }

    /// All offense kinds, for iteration.
    pub fn all() -> &'static [OffenseKind] {
        &[
            Self::ShuffleFirstVsSample,
            Self::SelectFirstVsDetect,
            Self::SelectLastVsReverseDetect,
            Self::ReverseEachVsReverseEach,
            Self::KeysEachVsEachKey,
            Self::MapFlattenVsFlatMap,
            Self::GsubVsTr,
            Self::SortVsSortBy,
            Self::FetchWithArgumentVsBlock,
            Self::HashMergeBangVsHashBrackets,
            Self::BlockVsSymbolToProc,
            Self::EachWithIndexVsWhile,
            Self::IncludeVsCoverOnRange,
            Self::ModuleEval,
            Self::RescueVsRespondTo,
            Self::ProcCallVsYield,
            Self::GetterVsAttrReader,
            Self::SetterVsAttrWriter,
            Self::ForLoopVsEach,
        ]
    }
}

impl OffenseKind {
    /// Look up an OffenseKind by its config key string.
    pub fn from_config_key(key: &str) -> Option<OffenseKind> {
        Self::all().iter().find(|k| k.config_key() == key).copied()
    }
}

impl fmt::Display for OffenseKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.config_key())
    }
}

/// A detected offense at a specific location, with an optional auto-fix.
#[derive(Debug, Clone)]
pub struct Offense {
    pub kind: OffenseKind,
    pub line: usize,
    pub fix: Option<Fix>,
}

impl Offense {
    pub fn new(kind: OffenseKind, line: usize) -> Self {
        Self {
            kind,
            line,
            fix: None,
        }
    }

    pub fn with_fix(kind: OffenseKind, line: usize, fix: Fix) -> Self {
        Self {
            kind,
            line,
            fix: Some(fix),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_returns_19_variants() {
        assert_eq!(OffenseKind::all().len(), 19);
    }

    #[test]
    fn config_key_roundtrips_via_from_config_key() {
        for kind in OffenseKind::all() {
            let key = kind.config_key();
            let restored = OffenseKind::from_config_key(key);
            assert_eq!(restored, Some(*kind), "roundtrip failed for {}", key);
        }
    }

    #[test]
    fn from_config_key_returns_none_for_unknown() {
        assert_eq!(OffenseKind::from_config_key("nonexistent"), None);
    }

    #[test]
    fn explanation_is_non_empty_for_all() {
        for kind in OffenseKind::all() {
            let explanation = kind.explanation();
            assert!(!explanation.is_empty(), "empty explanation for {:?}", kind);
        }
    }

    #[test]
    fn display_matches_config_key() {
        for kind in OffenseKind::all() {
            assert_eq!(format!("{}", kind), kind.config_key());
        }
    }

    #[test]
    fn offense_new_has_no_fix() {
        let offense = Offense::new(OffenseKind::GsubVsTr, 42);
        assert_eq!(offense.kind, OffenseKind::GsubVsTr);
        assert_eq!(offense.line, 42);
        assert!(offense.fix.is_none());
    }

    #[test]
    fn offense_with_fix_has_fix() {
        let fix = Fix::single(0, 5, "hello");
        let offense = Offense::with_fix(OffenseKind::ForLoopVsEach, 10, fix);
        assert_eq!(offense.kind, OffenseKind::ForLoopVsEach);
        assert_eq!(offense.line, 10);
        assert!(offense.fix.is_some());
    }
}
