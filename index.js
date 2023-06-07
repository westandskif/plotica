/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
import wasm from "./Cargo.toml";

function init() {
  window._wasm_graphima = window._wasm_graphima || wasm();
  return window._wasm_graphima;
}

async function createMain(params, config) {
  const exports = await init();
  return exports.createMain(params, config);
}
async function destroyMain(chartId) {
  const exports = await init();
  return exports.destroyMain(chartId);
}

export default {
  init, // optional
  createMain,
  destroyMain,
};
