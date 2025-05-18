import init, { createMain } from "../dist/index.js";

// --8<-- [start:main]
async function run() {
  await init();
  const response = await fetch("../../overview.json");
  const data = await response.json();

  const params = {
    selector: "#chart-3",
    contentName: "New chart",
    coordType: "date",
    valueType: "number",
    dataSets: [
      {
        name: data.names.y0,
        coords: data.columns[0].slice(1),
        values: data.columns[1].slice(1),
      },
      {
        name: `${data.names.y1} x 100`,
        coords: data.columns[0].slice(1),
        values: data.columns[2].slice(1).map((v) => v * 100.0),
      },
      {
        name: `${data.names.y1} x 1000`,
        coords: data.columns[0].slice(1),
        values: data.columns[2].slice(1).map((v) => v * 1000.0),
      },
    ],
  };
  let config = Object.create(CONFIG);
  config.autoLogScaleThreshold = 1500;
  createMain(params, config);
}
run();
// --8<-- [end:main]
