use std::collections::HashMap;
use std::path::Path;

use rubyfast::analyzer::analyze_file;
use rubyfast::config::Config;
use rubyfast::offense::OffenseKind::{self, *};

/// Every fixture with the complete set of offenses it must produce — no more, no less.
const FIXTURES: &[(&str, &[(OffenseKind, usize)])] = &[
    ("01_shuffle_first.rb", &[(ShuffleFirstVsSample, 3)]),
    ("02_select_first.rb", &[(SelectFirstVsDetect, 3)]),
    ("03_select_last.rb", &[(SelectLastVsReverseDetect, 1)]),
    ("04_reverse_each.rb", &[(ReverseEachVsReverseEach, 2)]),
    ("05_keys_each.rb", &[(KeysEachVsEachKey, 3)]),
    ("06_map_flatten.rb", &[(MapFlattenVsFlatMap, 2)]),
    ("07_gsub_vs_tr.rb", &[(GsubVsTr, 2)]),
    ("08_sort_vs_sort_by.rb", &[(SortVsSortBy, 1)]),
    (
        "09_fetch_with_argument.rb",
        &[(FetchWithArgumentVsBlock, 1)],
    ),
    // Only the interpolated string and the array literal have to be built.
    (
        "09b_fetch_frozen_strings.rb",
        &[(FetchWithArgumentVsBlock, 2)],
    ),
    ("10_hash_merge_bang.rb", &[(HashMergeBangVsHashBrackets, 3)]),
    ("11_block_vs_symbol_to_proc.rb", &[(BlockVsSymbolToProc, 3)]),
    ("12_each_with_index.rb", &[(EachWithIndexVsWhile, 1)]),
    ("13_include_vs_cover.rb", &[(IncludeVsCoverOnRange, 2)]),
    ("14_module_eval.rb", &[(ModuleEval, 1)]),
    ("14b_module_eval_heredoc.rb", &[(ModuleEval, 1)]),
    // 2 rescues with NoMethodError, 1 without
    ("15_rescue_vs_respond_to.rb", &[(RescueVsRespondTo, 2)]),
    // 2 methods with &block that call block.call, 1 without &block
    ("16_proc_call_vs_yield.rb", &[(ProcCallVsYield, 2)]),
    ("17_getter_vs_attr_reader.rb", &[(GetterVsAttrReader, 2)]),
    ("18_setter_vs_attr_writer.rb", &[(SetterVsAttrWriter, 2)]),
    ("19_for_loop.rb", &[(ForLoopVsEach, 1)]),
    // Only the offenses outside the disabled regions remain.
    (
        "20_inline_disable.rb",
        &[(ShuffleFirstVsSample, 2), (ForLoopVsEach, 1)],
    ),
    ("clean.rb", &[]),
];

fn analyze(fixture: &str) -> HashMap<OffenseKind, usize> {
    let path = Path::new("tests/fixtures").join(fixture);
    let result = analyze_file(&path, &Config::default()).unwrap();
    let mut counts = HashMap::new();
    for o in &result.offenses {
        *counts.entry(o.kind).or_default() += 1;
    }
    counts
}

#[test]
fn every_fixture_reports_exactly_its_expected_offenses() {
    for (fixture, expected) in FIXTURES {
        let expected: HashMap<_, _> = expected.iter().copied().collect();
        assert_eq!(analyze(fixture), expected, "{fixture}");
    }
}

#[test]
fn every_rule_is_caught_by_some_fixture() {
    let missing: Vec<_> = OffenseKind::all()
        .iter()
        .filter(|kind| {
            !FIXTURES
                .iter()
                .any(|(_, offenses)| offenses.iter().any(|(k, n)| k == *kind && *n > 0))
        })
        .collect();
    assert!(missing.is_empty(), "rules without a fixture: {missing:?}");
}

#[test]
fn every_fixture_file_is_listed() {
    let mut on_disk: Vec<_> = std::fs::read_dir("tests/fixtures")
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    on_disk.sort();
    let mut listed: Vec<_> = FIXTURES.iter().map(|(f, _)| f.to_string()).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
}
