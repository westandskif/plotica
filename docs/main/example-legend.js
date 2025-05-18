import init, { createMain } from "../dist/index.js";

// --8<-- [start:main]
async function run() {
  await init();
  const response = await fetch("../../overview.json");
  const data = await response.json();

  const params = {
    selector: "#chart-1",
    contentName: "New chart",
    coordType: "date",
    valueType: "number",
    dataSets: [
      {
        name: data.names.y0,
        coords: data.columns[0].slice(1), // list of coordinates
        values: data.columns[1].slice(1), // list of values
      },
      {
        name: data.names.y1,
        coords: data.columns[0].slice(1),
        values: data.columns[2].slice(1),
      },
      {
        name: "A name 1",
        coords: data.columns[0].slice(1),
        values: data.columns[1].slice(1).map((v) => v * 2),
      },
      {
        name: "B name 23",
        coords: data.columns[0].slice(1),
        values: data.columns[1].slice(1).map((v) => v * 3),
      },
      {
        name: "C name 234",
        coords: data.columns[0].slice(1),
        values: data.columns[1].slice(1).map((v) => v * 4),
      },
      {
        name: "D name 2345",
        coords: data.columns[0].slice(1),
        values: data.columns[1].slice(1).map((v) => v * 5),
      },
      {
        name: "E name 23456",
        coords: data.columns[0].slice(1),
        values: data.columns[1].slice(1).map((v) => v * 6),
      },
      {
        name: "F name 234567",
        coords: data.columns[0].slice(1),
        values: data.columns[1].slice(1).map((v) => v * 7),
      },
      {
        name: "G",
        coords: data.columns[0].slice(1),
        values: data.columns[1].slice(1).map((v) => v * 8),
      },
      {
        name: "H name",
        coords: data.columns[0].slice(1),
        values: data.columns[1].slice(1).map((v) => v * 9),
      },
    ],
  };
  createMain(params, CONFIG);
}
run();
// --8<-- [end:main]
