// Copyright 2020 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use common_datavalues::DataField;
use common_datavalues::DataValue;
use common_exception::ErrorCode;
use common_exception::Result;
use common_infallible::RwLock;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use unicase::UniCase;

use crate::aggregates::AggregateFunctionRef;
use crate::aggregates::Aggregators;

pub struct AggregateFunctionFactory;
pub type FactoryFunc = fn(
    name: &str,
    params: Vec<DataValue>,
    arguments: Vec<DataField>,
) -> Result<AggregateFunctionRef>;

pub type FactoryCombinatorFunc = fn(
    name: &str,
    params: Vec<DataValue>,
    arguments: Vec<DataField>,
    nested_func: FactoryFunc,
) -> Result<AggregateFunctionRef>;

type Key = UniCase<String>;
pub type FactoryFuncRef = Arc<RwLock<IndexMap<Key, FactoryFunc>>>;
pub type FactoryCombinatorFuncRef = Arc<RwLock<IndexMap<Key, FactoryCombinatorFunc>>>;

lazy_static! {
    static ref FACTORY: FactoryFuncRef = {
        let map: FactoryFuncRef = Arc::new(RwLock::new(IndexMap::new()));
        Aggregators::register(map.clone()).unwrap();

        map
    };
    static ref COMBINATOR_FACTORY: FactoryCombinatorFuncRef = {
        let map: FactoryCombinatorFuncRef = Arc::new(RwLock::new(IndexMap::new()));
        Aggregators::register_combinator(map.clone()).unwrap();
        map
    };
}

impl AggregateFunctionFactory {
    pub fn get(
        name: impl AsRef<str>,
        params: Vec<DataValue>,
        arguments: Vec<DataField>,
    ) -> Result<AggregateFunctionRef> {
        let name = name.as_ref();
        let not_found_error = || -> ErrorCode {
            ErrorCode::UnknownAggregateFunction(format!("Unsupported AggregateFunction: {}", name))
        };

        let key: Key = name.into();
        let map = FACTORY.read();
        match map.get(&key) {
            Some(creator) => (creator)(name, params, arguments),
            None => {
                // find suffix
                let lower_name = name.to_lowercase();
                let combinator = COMBINATOR_FACTORY.read();
                if let Some((k, &combinator_creator)) = combinator
                    .iter()
                    .find(|(c, _)| lower_name.ends_with(&c.to_lowercase()))
                {
                    let nested_name = lower_name
                        .strip_suffix(&k.to_lowercase())
                        .ok_or_else(not_found_error)?;
                    let nested_key: Key = nested_name.into();

                    return map
                        .get(&nested_key)
                        .map(|nested_creator| {
                            combinator_creator(nested_name, params, arguments, *nested_creator)
                        })
                        .unwrap_or_else(|| Err(not_found_error()));
                }

                Err(not_found_error())
            }
        }
    }

    pub fn check(name: impl AsRef<str>) -> bool {
        let name = name.as_ref();
        let key: Key = name.into();

        let map = FACTORY.read();

        if map.contains_key(&key) {
            return true;
        }

        // find suffix
        let lower_name = name.to_lowercase();
        let combinator = COMBINATOR_FACTORY.read();

        for (k, _) in combinator.iter() {
            if let Some(nested_name) = lower_name.strip_suffix(&k.to_lowercase()) {
                let nk: Key = nested_name.into();
                if map.contains_key(&nk) {
                    return true;
                }
            }
        }
        false
    }

    pub fn registered_names() -> Vec<String> {
        let map = FACTORY.read();
        map.keys().into_iter().map(|x| x.to_string()).collect()
    }
}
