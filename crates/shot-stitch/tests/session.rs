use shot_image::Image;
use shot_stitch::{Axis, Config, Edge, Status, Stitcher};

fn frame(origin: i32, width: i32, height: i32) -> Image {
    let mut pixels = Vec::new();
    for y in origin..origin + height {
        for x in 0..width {
            let hash = (y as u32).wrapping_mul(2_654_435_761).rotate_left(7)
                ^ (x as u32).wrapping_mul(2246822519);
            pixels.push(0xff000000 | (hash & 0x00ffffff));
        }
    }
    Image::from_pixels(width, height, pixels).unwrap()
}

fn config() -> Config {
    Config {
        min_overlap: 32,
        accept_diff: 0.5,
        min_append: 10,
        approx_diff: 0.1,
    }
}

#[test]
fn forward_revisit_and_prepend_keep_exact_content() {
    let mut stitch = Stitcher::new(config()).unwrap();
    assert_eq!(
        stitch.push(frame(100, 80, 180)).unwrap().status,
        Status::FirstFrame
    );
    let result = stitch.push(frame(145, 80, 180)).unwrap();
    assert_eq!(
        (result.status, result.added, result.edge),
        (Status::Appended, 45, Edge::End)
    );
    let result = stitch.push(frame(110, 80, 180)).unwrap();
    assert_eq!((result.status, result.position), (Status::NoProgress, 10));
    assert_eq!(
        stitch.push(frame(100, 80, 180)).unwrap().status,
        Status::NoProgress
    );
    let result = stitch.push(frame(60, 80, 180)).unwrap();
    assert_eq!(
        (result.status, result.added, result.edge),
        (Status::Appended, 40, Edge::Start)
    );
    assert_eq!(stitch.full_image().unwrap().unwrap(), frame(60, 80, 265));
    assert_eq!(stitch.stats().frame_count, 3);
}

#[test]
fn pending_motion_accumulates_against_original_anchor() {
    let mut stitch = Stitcher::new(config()).unwrap();
    stitch.push(frame(0, 80, 180)).unwrap();
    for origin in [3, 6, 9] {
        assert_eq!(
            stitch.push(frame(origin, 80, 180)).unwrap().status,
            Status::NoProgress
        );
    }
    let result = stitch.push(frame(12, 80, 180)).unwrap();
    assert_eq!((result.status, result.added), (Status::Appended, 12));
    assert_eq!(stitch.full_image().unwrap().unwrap(), frame(0, 80, 192));
}

#[test]
fn horizontal_stitch_and_axis_lock() {
    let mut stitch = Stitcher::new(config()).unwrap();
    assert!(stitch.set_axis(Axis::Horizontal).unwrap());
    stitch.push(frame(0, 64, 180).transpose().unwrap()).unwrap();
    assert!(!stitch.axis_locked());
    assert_eq!(
        stitch
            .push(frame(40, 64, 180).transpose().unwrap())
            .unwrap()
            .status,
        Status::Appended
    );
    assert!(!stitch.set_axis(Axis::Vertical).unwrap());
    assert_eq!(
        stitch.full_image().unwrap().unwrap(),
        frame(0, 64, 220).transpose().unwrap()
    );
}

#[test]
fn seed_axis_switch_preserves_image_and_cache() {
    let mut stitch = Stitcher::new(config()).unwrap();
    let seed = frame(0, 64, 180);
    stitch.push(seed.clone()).unwrap();
    assert!(stitch.set_axis(Axis::Horizontal).unwrap());
    assert_eq!(stitch.full_image().unwrap(), Some(seed.clone()));
    let revision = stitch.revision();
    assert_eq!(stitch.full_image().unwrap(), Some(seed));
    assert_eq!(revision, stitch.revision());
}

#[test]
fn rejected_frames_do_not_replace_anchor() {
    let mut stitch = Stitcher::new(config()).unwrap();
    stitch.push(frame(0, 80, 180)).unwrap();
    assert_eq!(
        stitch.push(frame(0, 81, 180)).unwrap().status,
        Status::NoMatch
    );
    assert_eq!(
        stitch.push(frame(10000, 80, 180)).unwrap().status,
        Status::NoMatch
    );
    assert_eq!(
        stitch.push(frame(40, 80, 180)).unwrap().status,
        Status::Appended
    );
    assert_eq!(stitch.full_image().unwrap().unwrap(), frame(0, 80, 220));
    let mut blank = Stitcher::new(config()).unwrap();
    let image = Image::from_pixels(80, 180, vec![0xffffffff; 80 * 180]).unwrap();
    blank.push(image.clone()).unwrap();
    assert_eq!(blank.push(image).unwrap().status, Status::NoProgress);
}

#[test]
fn config_and_session_isolation() {
    assert!(
        Stitcher::new(Config {
            min_overlap: i32::MAX,
            ..config()
        })
        .is_err()
    );
    assert!(
        Stitcher::new(Config {
            min_overlap: 0,
            ..config()
        })
        .is_err()
    );
    assert!(
        Stitcher::new(Config {
            accept_diff: f32::NAN,
            ..config()
        })
        .is_err()
    );
    let mut first = Stitcher::new(config()).unwrap();
    let second = Stitcher::new(config()).unwrap();
    first.push(frame(0, 80, 180)).unwrap();
    assert_eq!(second.stats().frame_count, 0);
}
