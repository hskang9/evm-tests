use std::collections::{HashMap, BTreeMap};
use std::rc::Rc;
use serde::{Serialize, Deserialize};
use primitive_types::{H160, H256, U256};
use evm::gasometer;
use evm::backend::{Backend, ApplyBackend, MemoryBackend, MemoryVicinity, MemoryAccount};
use evm::executor::StackExecutor;
use ethjson::vm::Vm;
use crate::utils::*;

#[derive(Deserialize, Debug)]
pub struct Test(Vm);

impl Test {
	pub fn unwrap_to_pre_state(&self) -> BTreeMap<H160, MemoryAccount> {
		unwrap_to_state(&self.0.pre_state)
	}

	pub fn unwrap_to_vicinity(&self) -> MemoryVicinity {
		MemoryVicinity {
			gas_price: self.0.transaction.gas_price.clone().into(),
			origin: self.0.transaction.origin.clone().into(),
			block_hashes: Vec::new(),
			block_number: self.0.env.number.clone().into(),
			block_coinbase: self.0.env.author.clone().into(),
			block_timestamp: self.0.env.timestamp.clone().into(),
			block_difficulty: self.0.env.difficulty.clone().into(),
			block_gas_limit: self.0.env.gas_limit.clone().into(),
		}
	}

	pub fn unwrap_to_code(&self) -> Rc<Vec<u8>> {
		Rc::new(self.0.transaction.code.clone().into())
	}

	pub fn unwrap_to_data(&self) -> Rc<Vec<u8>> {
		Rc::new(self.0.transaction.data.clone().into())
	}

	pub fn unwrap_to_context(&self) -> evm::Context {
		evm::Context {
			address: self.0.transaction.address.clone().into(),
			caller: self.0.transaction.sender.clone().into(),
			apparent_value: self.0.transaction.value.clone().into(),
		}
	}

	pub fn unwrap_to_return_value(&self) -> Vec<u8> {
		self.0.output.clone().unwrap().into()
	}

	pub fn unwrap_to_gas_limit(&self) -> usize {
		self.0.transaction.gas.clone().into()
	}

	pub fn unwrap_to_post_gas(&self) -> usize {
		self.0.gas_left.clone().unwrap().into()
	}
}

pub fn test(name: &str, test: Test) {
	use std::io::{self, Write};

	print!("Running test {} ... ", name);
	io::stdout().flush().ok().expect("Could not flush stdout");

	let original_state = test.unwrap_to_pre_state();
	let vicinity = test.unwrap_to_vicinity();
	let gasometer_config = gasometer::Config::frontier();
	let mut backend = MemoryBackend::new(&vicinity, original_state);
	let mut executor = StackExecutor::new(
		&backend,
		test.unwrap_to_gas_limit(),
		&gasometer_config,
	);

	let code = test.unwrap_to_code();
	let data = test.unwrap_to_data();
	let context = test.unwrap_to_context();
	let mut runtime = evm::Runtime::new(code, data, 1024, 1000000, context);

	let reason = executor.execute(&mut runtime);
	let (gas, values, logs) = executor.finalize();
	backend.apply(values, logs);

	if test.0.output.is_none() {
		print!("{:?} ", reason);

		assert!(reason.is_error());
		assert!(test.0.post_state.is_none() && test.0.gas_left.is_none());

		println!("succeed");
	} else {
		let expected_post_gas = test.unwrap_to_post_gas();
		print!("{:?} ", reason);

		assert_eq!(runtime.machine().return_value(), test.unwrap_to_return_value());
		assert_valid_state(test.0.post_state.as_ref().unwrap(), &backend.state());
		assert_eq!(gas, expected_post_gas);
		println!("succeed");
	}
}