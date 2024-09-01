use tket2::serialize::save_tk1_json_str;
use tket2::{static_circ::StaticSizeCircuit, Circuit};

use super::ViewSerialise;

impl ViewSerialise for StaticSizeCircuit {
    fn graph_type(&self) -> &'static str {
        "tket"
    }

    fn to_json(&self) -> String {
        let tket_circ: Circuit = self.clone().into();
        save_tk1_json_str(&tket_circ).unwrap()
    }
}
