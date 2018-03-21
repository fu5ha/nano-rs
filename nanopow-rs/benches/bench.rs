#[macro_use]
extern crate criterion;
extern crate nanopow_rs;

use criterion::Criterion;

use nanopow_rs::{generate_work, InputHash};

fn generate_unlimited(c: &mut Criterion) {
    c.bench_function("generate unlimited", |b| {
      let hash = InputHash::from_hex("47F694A96653EB497709490776E492EFBB88EBC5C4E95CC0B2C9DCAB1930C36B").unwrap();
      b.iter(|| generate_work(&hash, None).unwrap())
    });
}

criterion_group!{
  name = benches;
  config = Criterion::default().sample_size(10);
  targets = generate_unlimited
}
criterion_main!(benches);