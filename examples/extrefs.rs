use bls2brz::brdb::{Brz, IntoReader};
fn main() {
    let p = std::env::args().nth(1).unwrap();
    let db = Brz::open(&p).unwrap().into_reader();
    let gd = db.global_data().unwrap();
    println!("types: {:?}", gd.external_asset_types);
    println!("refs: {:?}", gd.external_asset_references);
}
