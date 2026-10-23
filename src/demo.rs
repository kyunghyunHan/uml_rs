//! A richer review fixture: multilingual labels, all layouts and stable cross-diagram links.
use crate::model::*;
pub fn review_project() -> Project {
    let mut p = crate::app::example_project();
    p.name = "정렬 프로그램 · 개선 검증".into();
    let class = &mut p.diagrams[1];
    class.nodes[0].name = "정렬 요청을 처리하는 SortService".into();
    class.nodes[0].methods[0].parameters =
        "values: List<int>, policy: SortPolicy, options: 정렬실행설정".into();
    class.nodes[0].reflow();
    class.nodes[1].methods[0].parameters = "values: List<int>, comparison: 비교정책".into();
    class.nodes[1].reflow();
    class.nodes[2].name = "안정적인 버블 정렬 BubbleSort".into();
    class.nodes[2].methods[0].parameters = "values: List<int>, comparison: 비교정책".into();
    class.nodes[2].reflow();
    class.nodes[0].layout.pos = Point::new(80., 80.);
    class.nodes[1].layout.pos = Point::new(850., 80.);
    class.nodes[2].layout.pos = Point::new(850., 410.);
    let service = class.nodes[0].id;
    let service_method = class.nodes[0].methods[0].id;
    let policy = class.nodes[2].id;
    let policy_method = class.nodes[2].methods[0].id;
    let mut buffer = Node::new(NodeKind::Class, Point::new(80., 410.));
    buffer.name = "SortBuffer".into();
    buffer.hide_empty = true;
    buffer.attributes.push(Member {
        name: "capacity".into(),
        type_name: "usize".into(),
        value: "1024".into(),
        visibility: "-".into(),
        ..Default::default()
    });
    buffer.reflow();
    let buffer_id = buffer.id;
    let mut note = Node::new(NodeKind::Note, Point::new(440., 520.));
    note.name =
        "동일한 값의 상대 순서를 유지합니다.\n정렬 정책은 인터페이스로 교체할 수 있습니다.".into();
    note.layout.size = Point::new(320., 135.);
    class
        .relations
        .push(Relation::new(note.id, policy, RelationKind::Comment));
    class.nodes.extend([buffer, note]);
    let mut package = Node::new(NodeKind::Package, Point::new(40., 20.));
    package.name = "정렬 핵심".into();
    package.layout.size = Point::new(1450., 690.);
    let pkg = package.id;
    for n in &mut class.nodes {
        n.boundary = Some(pkg);
    }
    class.nodes.insert(0, package);
    let seq = &mut p.diagrams[2];
    seq.name = "정렬 실행 · 중첩과 생성/소멸".into();
    let user = Participant {
        id: id(),
        name: "사용자".into(),
        actor: true,
        type_ref: None,
    };
    let svc = Participant {
        id: id(),
        name: String::new(),
        actor: false,
        type_ref: Some(service),
    };
    let pol = Participant {
        id: id(),
        name: String::new(),
        actor: false,
        type_ref: Some(policy),
    };
    let buf = Participant {
        id: id(),
        name: String::new(),
        actor: false,
        type_ref: Some(buffer_id),
    };
    let message = |source, target, kind, text: &str, method_ref| Message {
        id: id(),
        source,
        target,
        kind,
        text: text.into(),
        method_ref,
    };
    let messages = vec![
        message(user.id, svc.id, MessageKind::Sync, "", Some(service_method)),
        message(svc.id, buf.id, MessageKind::Create, "작업 버퍼 생성", None),
        message(svc.id, pol.id, MessageKind::Sync, "", Some(policy_method)),
        message(
            pol.id,
            pol.id,
            MessageKind::Sync,
            "비교할 인접 원소를 선택하고 현재 정렬 상태와 경계 조건을 검사합니다",
            None,
        ),
        message(
            pol.id,
            pol.id,
            MessageKind::Sync,
            "왼쪽 값이 더 크면 두 원소의 위치를 교환합니다",
            None,
        ),
        message(
            pol.id,
            pol.id,
            MessageKind::Return,
            "교환 결과와 다음 비교 위치를 반환",
            None,
        ),
        message(
            pol.id,
            svc.id,
            MessageKind::Return,
            "정렬된 결과: [1, 2, 3, 5, 8, 13]",
            None,
        ),
        message(svc.id, buf.id, MessageKind::Destroy, "작업 버퍼 해제", None),
        message(
            svc.id,
            user.id,
            MessageKind::Return,
            "정렬 완료 · 결과를 표시합니다",
            None,
        ),
    ];
    let event = |i: usize| messages[i].id;
    seq.sequence = Sequence {
        participants: vec![user.clone(), svc.clone(), pol.clone(), buf],
        activations: vec![
            Activation {
                id: id(),
                participant: svc.id,
                start: event(0),
                end: event(8),
                automatic: false,
            },
            Activation {
                id: id(),
                participant: pol.id,
                start: event(2),
                end: event(6),
                automatic: false,
            },
            Activation {
                id: id(),
                participant: pol.id,
                start: event(3),
                end: event(5),
                automatic: false,
            },
            Activation {
                id: id(),
                participant: pol.id,
                start: event(4),
                end: event(5),
                automatic: false,
            },
        ],
        fragments: vec![
            Fragment {
                id: id(),
                kind: FragmentKind::Alt,
                condition: "입력값이 있고 선택한 정책으로 정렬할 수 있는 경우".into(),
                else_condition: "정렬 결과가 준비됨".into(),
                start: event(2),
                end: event(6),
                split: Some(event(6)),
                participant_start: Some(svc.id),
                participant_end: Some(pol.id),
            },
            Fragment {
                id: id(),
                kind: FragmentKind::Loop,
                condition: "앞선 반복에서 교환이 발생하는 동안".into(),
                else_condition: String::new(),
                start: event(3),
                end: event(5),
                split: None,
                participant_start: Some(pol.id),
                participant_end: Some(pol.id),
            },
        ],
        messages,
        auto_activate: false,
    };
    let sequence_id = seq.id;
    let usecase = &mut p.diagrams[0];
    let mut expert = Node::new(NodeKind::Actor, Point::new(25., 400.));
    expert.name = "고급 설정을 사용하는 관리자".into();
    expert.layout.size = Point::new(145., 220.);
    let parent = usecase.nodes[1].id;
    usecase.relations.push(Relation::new(
        expert.id,
        parent,
        RelationKind::Generalization,
    ));
    usecase.relations.push(Relation::new(
        expert.id,
        usecase.nodes[3].id,
        RelationKind::Association,
    ));
    usecase.nodes.push(expert);
    usecase.nodes[0].layout.size = Point::new(650., 460.);
    usecase.nodes[2].name = "선택한 정책으로 입력값 정렬 실행".into();
    usecase.nodes[2].layout.size = Point::new(240., 100.);
    usecase.nodes[3].layout.pos = Point::new(590., 290.);
    let spec = &mut usecase.nodes[2].specification;
    spec.goal = "선택한 정책에 따라 입력 목록을 정렬한다".into();
    spec.primary_actor = "사용자".into();
    spec.preconditions = "정수 목록과 정렬 정책이 준비되어 있다".into();
    spec.basic_flow="1. 사용자가 정렬을 요청한다.\n2. 서비스가 정책의 sort 메서드를 호출한다.\n3. 결과를 사용자에게 표시한다.".into();
    spec.alternatives = "빈 입력이면 빈 결과를 반환한다.".into();
    spec.postconditions = "결과 목록이 오름차순이며 원소가 보존된다.".into();
    spec.sequences.push(sequence_id);
    p.synchronize_links();
    p
}
