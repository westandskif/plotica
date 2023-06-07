async function run() {
  const response = await fetch("./overview.json");
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
    ],
  };
  Graphima.createMain(params, CONFIG);
}

const CONFIG = {
  version: 1,
  fontStandard: "system-ui",
  fontMonospace: "monospace",
  fontSizeSmall: 10,
  fontSizeNormal: 12,
  fontSizeLarge: 14,
  fontWidthCoeff: 0.65,
  lineWidth: 1.5,
  circleRadius: 2,
  colorGrid: [237, 237, 237],
  colorTick: [142, 142, 142],
  colorCameraGrip: [0, 0, 255, 0.15],
  colorPreviewOverlay: [0, 0, 0, 0.4],
  colorPreviewHint: [255, 255, 255, 1],
  colorTooltip: [255, 255, 255, 1],
  colorTooltipFont: [0, 0, 0, 1],
  sortDataSetsBy: "medianDesc",
  layoutContentHeight: 5,
  layoutPreviewHeight: 1,
  layoutLegendHeight: 1.5,
  colorPalette: [
    [75, 216, 100],
    [254, 60, 47],
    [147, 12, 249],
    [54, 152, 224],
    [255, 221, 50],

    // same, but 1.7x darker
    [44, 127, 58],
    [149, 35, 27],
    [86, 7, 146],
    [31, 89, 131],
    [150, 130, 29],
  ],
  msLongPress: 500,
  // switch to pseudo-log scale if series cover 15x more vertical space than
  // when linear scale is used
  autoLogScaleThreshold: 15,
  expFmtSignificantDigits: 5,
};
run();
