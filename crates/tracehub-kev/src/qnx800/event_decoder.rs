//! イベントの表示文字列(traceprinter -n と同じ書式)。
//!
//! 書式はすべて args.rs から来る。ここにあるのは「どの書式をどう当てはめるか」という手順だけ。
//! - 内部 ID をセマンティック ID にして、args.rs の EVENTS の行を引く(定義の無い番号はクラスの any の行)
//! - S と、CB だけで途切れた組(payloads が 2 語以下)は fast の書式、それ以外の組は wide の書式
//! - KER_CALL の表示名はコール番号と種別から、スレッドの状態の表示名は状態の名前から決まる
//! - SYSTEM の SLOG は、payloads が slog2 のレコードなら SLOG2(slog2.rs)

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use super::{args, def, semantic_ids};
use crate::semantic_id::{ClassId, EventId};
use crate::tp_format::{self, CItem, Cond, Missing, Spec, Table, Template, Var};

/// 1 行の部品
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    pub class: String,
    pub name: String,
    /// 名前より後ろ(先頭の区切りの空白を含む)
    pub args: String,
}

#[derive(Debug)]
enum CSpec {
    None,
    Silent,
    Items(Vec<CItem>),
}

fn cspec(s: Spec) -> CSpec {
    match s {
        Spec::None => CSpec::None,
        Spec::Silent => CSpec::Silent,
        Spec::Items(i) => CSpec::Items(tp_format::compile(i)),
    }
}

#[derive(Debug)]
struct CEv {
    name: Option<Template>,
    fast: CSpec,
    wide: CSpec,
    min_words: usize,
    intcall: bool,
}

/// args.rs をコンパイルしたもの
struct Spec800 {
    events: HashMap<(ClassId, EventId), CEv>,
    /// クラスの定義の無い番号の行と、内部イベント番号の上限
    any: HashMap<ClassId, (CEv, Option<u16>)>,
    blank: HashSet<(ClassId, EventId)>,
    tables: HashMap<&'static str, Table>,
    class_display: HashMap<ClassId, &'static str>,
    fast_fallback_wide: HashMap<ClassId, bool>,
    kc_names: HashMap<u16, &'static str>,
    kc_names_exit: HashMap<u16, &'static str>,
    kc_raw_names: HashMap<u16, &'static str>,
    kc_name: Template,
    kc_unknown_call: Template,
    intcall_name: Template,
    intcall_unknown_call: Template,
    failure_when: Cond,
    failure: Vec<CItem>,
    th_name: Template,
    slog2: (ClassId, EventId),
}

fn spec() -> &'static Spec800 {
    static S: OnceLock<Spec800> = OnceLock::new();
    S.get_or_init(build)
}

fn build() -> Spec800 {
    let mut events = HashMap::new();
    let mut any = HashMap::new();
    for e in args::EVENTS {
        let compiled = CEv {
            name: e.name.map(Template::parse),
            fast: cspec(e.fast),
            wide: cspec(e.wide),
            min_words: e.min_words as usize,
            intcall: e.intcall,
        };
        match e.event {
            Some(id) => assert!(
                events.insert((e.class, id), compiled).is_none(),
                "args.rs: duplicate definition of {:?} {:?}",
                e.class,
                id
            ),
            None => assert!(
                any.insert(e.class, (compiled, e.max_code)).is_none(),
                "args.rs: duplicate any({:?})",
                e.class
            ),
        }
    }

    let kc_num = |n: &str| {
        def::KERCALLS
            .iter()
            .find(|r| r.1 == n)
            .unwrap_or_else(|| panic!("args.rs: unknown kernel call {n}"))
            .0
    };
    let mut kc_names: HashMap<u16, &'static str> = def::KERCALLS.iter().map(|r| (r.0, r.1)).collect();
    for (n, d) in args::KERCALL_NAMES {
        kc_names.insert(kc_num(n), d);
    }

    Spec800 {
        events,
        any,
        blank: args::BLANK_AFTER.iter().copied().collect(),
        tables: args::TABLES.iter().copied().collect(),
        class_display: args::CLASS_DISPLAY.iter().copied().collect(),
        fast_fallback_wide: args::FAST_FALLBACK_WIDE.iter().copied().collect(),
        kc_names,
        kc_names_exit: args::KERCALL_NAMES_EXIT.iter().map(|(n, d)| (kc_num(n), *d)).collect(),
        kc_raw_names: def::KERCALLS.iter().map(|r| (r.0, r.1)).collect(),
        kc_name: Template::parse(args::KERCALL_NAME),
        kc_unknown_call: Template::parse(args::KERCALL_UNKNOWN_CALL),
        intcall_name: Template::parse(args::INTCALL_NAME),
        intcall_unknown_call: Template::parse(args::INTCALL_UNKNOWN_CALL),
        failure_when: Cond::parse(args::KERCALL_FAILURE_WHEN),
        failure: tp_format::compile(args::KERCALL_FAILURE),
        th_name: Template::parse(args::THREAD_NAME),
        slog2: args::SLOG2_EVENT,
    }
}

/// 定義の誤りを確かめる(試験用。定義が読めなければ panic)
#[cfg(test)]
pub(crate) fn check_spec() {
    let _ = spec();
}

/// args.rs の EVENTS にある (class_id, event_id)(試験用)
#[cfg(test)]
pub(crate) fn defined_ids() -> Vec<(ClassId, EventId)> {
    spec().events.keys().copied().collect()
}

fn is_kercall(c: ClassId) -> bool {
    matches!(
        c,
        ClassId::KER_CALL_ENTER | ClassId::KER_CALL_EXIT | ClassId::KER_CALL_INT
    )
}

/// 内部 ID の行: EVENTS の行、無ければクラスの any の行(上限の番号まで)
fn row(sp: &Spec800, class: u8, code: u16) -> Option<(ClassId, EventId, &CEv)> {
    let (c, e) = semantic_ids(class, code);
    if let Some(r) = sp.events.get(&(c, e)) {
        return Some((c, e, r));
    }
    let (r, max) = sp.any.get(&c)?;
    if max.is_some_and(|m| code > m) {
        return None;
    }
    Some((c, e, r))
}

/// 書式の選択: S(と CB だけで途切れた組)は fast、組は wide
fn choose<'a>(sp: &Spec800, class: ClassId, multi: bool, ev: &'a CEv) -> Option<&'a CSpec> {
    if multi {
        return Some(&ev.wide);
    }
    match &ev.fast {
        CSpec::None => sp
            .fast_fallback_wide
            .get(&class)
            .copied()
            .unwrap_or(args::FAST_FALLBACK_WIDE_DEFAULT)
            .then_some(&ev.wide),
        s => Some(s),
    }
}

/// def.rs の名前(64 ビット版は 64 ビットでない版の名前)
fn def_name(class: u8, code: u16) -> Option<&'static str> {
    let tables: &[&[(u16, &str, EventId)]] = match class {
        def::CLASS_CONTROL => &[def::CONTROL_EVENTS],
        def::CLASS_INT => &[def::INT_EVENTS],
        def::CLASS_PR_TH => &[def::THREAD_STATES, def::PR_TH_VTHREAD, def::PR_TH_PROCESS],
        def::CLASS_SYSTEM => &[def::SYSTEM_EVENTS, def::SYSTEM_EVENTS_64],
        def::CLASS_COMM => &[def::COMM_EVENTS],
        def::CLASS_SEC => &[def::SEC_EVENTS],
        def::CLASS_QVM => &[def::QVM_EVENTS],
        _ => &[],
    };
    tables.iter().flat_map(|t| t.iter()).find(|r| r.0 == code).map(|r| r.1)
}

/// 表示クラス名と表示イベント名。行の無いもの(KER_CALL 以外)は名前が決まらない
fn display(sp: &Spec800, c: ClassId, class: u8, code: u16, ev: Option<&CEv>) -> (Option<&'static str>, Option<String>) {
    if is_kercall(c) {
        let intcall = ev.is_some_and(|e| e.intcall);
        let num = code & def::KERCALL_NUM_MASK;
        let mut name = String::new();
        let mut call = String::new();
        if intcall {
            match sp.kc_raw_names.get(&num) {
                Some(n) => call.push_str(n),
                None => sp
                    .intcall_unknown_call
                    .render_vars(&[("num", Var::Num(num as u64))], &mut call),
            }
            sp.intcall_name
                .render_vars(&[("call", Var::Str(&call)), ("num", Var::Num(num as u64))], &mut name);
            return (Some(args::DISPLAY_INT_CALL), Some(name));
        }
        let exit = c == ClassId::KER_CALL_EXIT;
        match if exit { sp.kc_names_exit.get(&num) } else { None }.or_else(|| sp.kc_names.get(&num)) {
            Some(n) => call.push_str(n),
            None => sp
                .kc_unknown_call
                .render_vars(&[("num", Var::Num(num as u64))], &mut call),
        }
        sp.kc_name
            .render_vars(&[("call", Var::Str(&call)), ("num", Var::Num(num as u64))], &mut name);
        return (sp.class_display.get(&c).copied(), Some(name));
    }
    let disp = sp.class_display.get(&c).copied();
    let ev = match ev {
        Some(e) => e,
        None => return (disp, None),
    };
    let mut name = String::new();
    match &ev.name {
        Some(t) => t.render_vars(&[("code", Var::Num(code as u64))], &mut name),
        None if c == ClassId::THREAD && code < def::PR_TH_PROCESS_BASE => sp.th_name.render_vars(
            &[("name", Var::Str(def_name(class, code).unwrap_or_default()))],
            &mut name,
        ),
        None => name.push_str(def_name(class, code).unwrap_or_default()),
    }
    (disp, Some(name))
}

/// 1 イベントの行。traceprinter が行を出さないなら None
///
/// - words: payloads(Truncated・Orphan なら取り出したもの)。2 語より多ければ組として扱う
pub(crate) fn format(class: u8, code: u16, words: &[u32], missing: Missing) -> Option<Line> {
    let sp = spec();
    let (c, e, ev) = row(sp, class, code)?;
    let (disp, name) = display(sp, c, class, code, Some(ev));
    let (class_name, name) = (disp?.to_string(), name.unwrap_or_default());
    if (c, e) == sp.slog2
        && let Some(a) = super::slog2::render(words)
    {
        return Some(Line {
            class: class_name,
            name: args::SLOG2_NAME.into(),
            args: a,
        });
    }
    if ev.intcall {
        return Some(Line {
            class: class_name,
            name,
            args: String::new(),
        });
    }
    if words.len() < ev.min_words {
        return None;
    }
    let CSpec::Items(items) = choose(sp, c, words.len() > 2, ev)? else {
        return None;
    };
    let mut a = String::new();
    // 失敗した exit は共通の書式(表示するイベントに限る)
    if c == ClassId::KER_CALL_EXIT && sp.failure_when.eval(words) {
        tp_format::render(&sp.failure, words, &sp.tables, missing, &mut a);
    } else {
        tp_format::render(items, words, &sp.tables, missing, &mut a);
    }
    Some(Line {
        class: class_name,
        name,
        args: a,
    })
}

/// traceprinter がイベントの後ろに空行を入れるか(行を出さないイベントでも入れる)
pub(crate) fn blank_after(class: u8, code: u16) -> bool {
    spec().blank.contains(&semantic_ids(class, code))
}

/// 表示クラス名と表示イベント名(行を出さないイベントでも、名前の規則で決まれば返す)
pub(crate) fn names(class: u8, code: u16) -> (Option<String>, Option<String>) {
    let sp = spec();
    let (c, _) = semantic_ids(class, code);
    let ev = row(sp, class, code).map(|r| r.2);
    let (disp, name) = display(sp, c, class, code, ev);
    (disp.map(String::from), name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(class: u8, code: u16, w: &[u32]) -> Option<String> {
        format(class, code, w, Missing::Omit).map(|l| format!("{}|{}|{}", l.class, l.name, l.args))
    }

    #[test]
    fn spec_compiles() {
        check_spec();
    }

    #[test]
    fn formats() {
        let aaaa = 0x41414141;
        // fast と wide は payloads の語数で選ぶ
        assert_eq!(
            line(def::CLASS_KER_CALL, 11, &[3, aaaa]).unwrap(),
            "KER_CALL|MSG_SENDV/11| coid:0x00000003 msg[0]:\"AAAA\" (0x41414141)"
        );
        assert_eq!(
            line(def::CLASS_KER_CALL, 11, &[3, 1, 2, aaaa, 0x42424242, 0x43434343]).unwrap(),
            "KER_CALL|MSG_SENDV/11| coid:0x00000003 sparts:1 rparts:2 msg[0]:\"AAAABBBBCCCC\" (0x41414141 0x42424242 0x43434343)"
        );
        assert_eq!(
            line(def::CLASS_PR_TH, 1, &[1, 6]).unwrap(),
            "THREAD|THRUNNING| pid:1 tid:6 "
        );
        assert_eq!(
            line(def::CLASS_PR_TH, 1, &[1, 6, 10, 2]).unwrap(),
            "THREAD|THRUNNING| pid:1 tid:6 priority:10 policy:2"
        );
        assert_eq!(
            line(def::CLASS_SYSTEM, 12, &[3, 5, 0x6261, 0]).unwrap(),
            "SYSTEM|SLOG| opcode:3 severity:5 message:ab"
        );
        assert_eq!(
            line(def::CLASS_CONTROL, 2, &[1, 6]).unwrap(),
            "CONTROL|BUFFER| sequence = 1, num_events = 6"
        );
        assert_eq!(line(0, 0, &[0, 0]), None);
    }
}
