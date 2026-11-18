//! Questions about tags: nearest first with ties by ID, the nearest or
//! nothing, flat distance in 2D against 3D, the grid always agreeing with
//! a look through every member, and the scale case's own share.

use super::*;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// A view of one tag, Enemy, on `places` (entity `i + 1` at each).
fn enemies(places: &[Vec3], flat: bool) -> (TagView, TagId) {
    let enemy = TagId::random();
    let tagged: Vec<(EntityId, Vec3, Vec<TagId>)> = places
        .iter()
        .enumerate()
        .map(|(i, p)| (id(i as u64 + 1), *p, vec![enemy]))
        .collect();
    (
        TagView::new(&[(enemy, "Enemy".into())], &tagged, flat),
        enemy,
    )
}

/// What a look through every member gives, to hold the grid to.
fn by_hand(places: &[Vec3], at: Vec3, radius: f32, flat: bool) -> Vec<EntityId> {
    let mut found: Vec<(f32, EntityId)> = places
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let d = if flat {
                p.truncate().distance_squared(at.truncate())
            } else {
                p.distance_squared(at)
            };
            (d, id(i as u64 + 1))
        })
        .filter(|(d, _)| *d <= radius * radius)
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    found.into_iter().map(|(_, e)| e).collect()
}

#[test]
fn within_comes_nearest_first_with_ties_by_id() {
    let places = [
        Vec3::new(3.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(9.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
    ];
    let (view, _) = enemies(&places, false);
    // 2, 3 and 5 are each 1 m away: by ID; then 1 at 3 m; 4 is too far.
    assert_eq!(
        view.within("Enemy", Vec3::ZERO, 3.0),
        [id(2), id(3), id(5), id(1)]
    );
    assert_eq!(view.within("Enemy", Vec3::ZERO, 0.5), []);
    assert_eq!(view.within("Enemy", Vec3::ZERO, -1.0), []);
    assert_eq!(view.within("Enemy", Vec3::ZERO, f32::NAN), []);
    assert_eq!(view.within("Enemy", Vec3::NAN, 3.0), []);
    assert_eq!(view.within("Nobody", Vec3::ZERO, 3.0), []);
    // A radius larger than the world gives every one.
    assert_eq!(view.within("Enemy", Vec3::ZERO, 1e30).len(), 5);
}

#[test]
fn nearest_is_the_nearest_or_nothing() {
    let places = [Vec3::new(3.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0)];
    let (view, _) = enemies(&places, false);
    assert_eq!(view.nearest("Enemy", Vec3::ZERO), Some(id(2)));
    assert_eq!(view.nearest("Enemy", Vec3::new(3.0, 1.0, 0.0)), Some(id(1)));
    // A tie goes to the smaller ID.
    let (view, _) = enemies(&[Vec3::X, Vec3::NEG_X], false);
    assert_eq!(view.nearest("Enemy", Vec3::ZERO), Some(id(1)));
    // None carrying it, a tag the project hasn't, a place that isn't one.
    let (empty, _) = enemies(&[], false);
    assert_eq!(empty.nearest("Enemy", Vec3::ZERO), None);
    assert_eq!(view.nearest("Nobody", Vec3::ZERO), None);
    assert_eq!(
        view.nearest("Enemy", Vec3::new(f32::INFINITY, 0.0, 0.0)),
        None
    );
}

#[test]
fn a_2d_project_measures_across_the_ground() {
    // A bat 50 m up (a layer, in 2D) and a slug 2 m away on the ground.
    let places = [Vec3::new(1.0, 0.0, 50.0), Vec3::new(2.0, 0.0, 0.0)];
    let (flat, _) = enemies(&places, true);
    let (solid, _) = enemies(&places, false);
    assert_eq!(flat.within("Enemy", Vec3::ZERO, 3.0), [id(1), id(2)]);
    assert_eq!(flat.nearest("Enemy", Vec3::ZERO), Some(id(1)));
    assert_eq!(solid.within("Enemy", Vec3::ZERO, 3.0), [id(2)]);
    assert_eq!(solid.nearest("Enemy", Vec3::ZERO), Some(id(2)));
}

#[test]
fn has_tag_knows_only_the_project_s_tags() {
    let (enemy, gone) = (TagId::random(), TagId::random());
    let tagged = vec![(id(1), Vec3::ZERO, vec![enemy, gone])];
    let view = TagView::new(&[(enemy, "Enemy".into())], &tagged, false);
    assert!(view.has(id(1), "Enemy"));
    assert!(!view.has(id(1), "Gone"));
    assert!(!view.has(id(2), "Enemy"));
    assert_eq!(view.tag("Enemy"), Some(enemy));
    assert_eq!(
        view.tag("enemy"),
        None,
        "names are exact, as game code's are"
    );
}

#[test]
fn the_grid_agrees_with_a_look_through_every_member_on_a_crowd() {
    // A crowd of 2,000 in a 200 m square, with clumps and a far one.
    let mut places: Vec<Vec3> = (0..2_000)
        .map(|i| {
            let t = i as f32 * 0.618_034;
            Vec3::new((t * 97.0) % 200.0, (t * 61.0) % 200.0, (i % 7) as f32)
        })
        .collect();
    places.extend((0..100).map(|i| Vec3::new(50.0 + i as f32 * 0.01, 50.0, 0.0)));
    places.push(Vec3::new(10_000.0, -10_000.0, 0.0));
    for flat in [false, true] {
        let (view, _) = enemies(&places, flat);
        assert!(view.members.values().next().unwrap().grid.is_some());
        for (n, at) in [
            Vec3::ZERO,
            Vec3::new(50.0, 50.0, 0.0),
            Vec3::new(123.4, 77.7, 3.0),
            Vec3::new(-500.0, 900.0, 0.0),
            Vec3::new(10_000.0, -9_990.0, 0.0),
        ]
        .into_iter()
        .enumerate()
        {
            for radius in [0.0, 0.5, 3.0, 20.0, 150.0, 1e6] {
                assert_eq!(
                    view.within("Enemy", at, radius),
                    by_hand(&places, at, radius, flat),
                    "within {radius} of {at} ({n}, flat {flat})"
                );
            }
            assert_eq!(
                view.nearest("Enemy", at),
                by_hand(&places, at, f32::INFINITY, flat).first().copied(),
                "nearest to {at}"
            );
        }
    }
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(48))]
    /// On any world, the grid's answers are a look through every member's.
    #[test]
    fn the_grid_matches_looking_through_everyone(
        places in proptest::collection::vec((-60.0f32..60.0, -60.0f32..60.0, -5.0f32..5.0), 0..300),
        at in (-80.0f32..80.0, -80.0f32..80.0, -5.0f32..5.0),
        radius in 0.0f32..90.0,
        flat in proptest::bool::ANY,
    ) {
        let places: Vec<Vec3> = places.into_iter().map(|(x, y, z)| Vec3::new(x, y, z)).collect();
        let at = Vec3::new(at.0, at.1, at.2);
        let (view, _) = enemies(&places, flat);
        let all = by_hand(&places, at, f32::INFINITY, flat);
        proptest::prop_assert_eq!(view.within("Enemy", at, radius), by_hand(&places, at, radius, flat));
        proptest::prop_assert_eq!(view.nearest("Enemy", at), all.first().copied());
    }
}

#[test]
fn answers_are_the_same_whatever_order_the_members_come_in() {
    let enemy = TagId::random();
    let places: Vec<(EntityId, Vec3, Vec<TagId>)> = (1..200)
        .map(|n| {
            (
                id(n),
                Vec3::new((n % 13) as f32, (n % 7) as f32, 0.0),
                vec![enemy],
            )
        })
        .collect();
    let mut shuffled = places.clone();
    shuffled.reverse();
    let tags = [(enemy, "Enemy".to_string())];
    let a = TagView::new(&tags, &places, false);
    let b = TagView::new(&tags, &shuffled, false);
    assert_eq!(
        a.within("Enemy", Vec3::ZERO, 5.0),
        b.within("Enemy", Vec3::ZERO, 5.0)
    );
    assert_eq!(a.nearest("Enemy", Vec3::ONE), b.nearest("Enemy", Vec3::ONE));
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_2000_units_asking_nearest_and_within_each_tick() {
    // Stage 1's scale case: 2,000 tagged units in a 200 m square, half of
    // them enemies, moving every tick; the view made once a tick, and
    // each unit asking for the nearest enemy and every enemy within 3 m.
    // Its share of a 60th of a second.
    const TARGET_MS: f64 = 2.0;
    let (enemy, unit) = (TagId::random(), TagId::random());
    let tags = [(enemy, "Enemy".to_string()), (unit, "Unit".to_string())];
    let start: Vec<Vec3> = (0..2_000)
        .map(|i| {
            let t = i as f32 * 0.618_034;
            Vec3::new((t * 97.0) % 200.0, (t * 61.0) % 200.0, 0.0)
        })
        .collect();
    let ticks = 120;
    let mut spent = std::time::Duration::ZERO;
    let mut found = 0usize;
    for tick in 0..ticks {
        let wobble = Vec3::new((tick as f32 * 0.1).sin(), (tick as f32 * 0.1).cos(), 0.0);
        let tagged: Vec<(EntityId, Vec3, Vec<TagId>)> = start
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let carried = if i % 2 == 0 {
                    vec![enemy, unit]
                } else {
                    vec![unit]
                };
                (id(i as u64 + 1), *p + wobble * (i % 5) as f32, carried)
            })
            .collect();
        let began = std::time::Instant::now();
        let view = TagView::new(&tags, &tagged, false);
        for (_, place, _) in &tagged {
            found += usize::from(view.nearest("Enemy", *place).is_some());
            found += view.within("Enemy", *place, 3.0).len();
        }
        spent += began.elapsed();
    }
    let ms = spent.as_secs_f64() * 1000.0 / f64::from(ticks);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: 2,000 tagged units each asking nearest and within 3 m, the view made \
         once a tick {ms:.3} ms a tick ({found} answers; target {TARGET_MS} ms) {verdict}"
    );
    assert!(found > 0);
    assert!(ms <= TARGET_MS, "tags' questions take {ms:.3} ms a tick");
}
