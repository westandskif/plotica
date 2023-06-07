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
  colorGrid: [237, 237, 237], // rgb
  colorTick: [142, 142, 142],
  colorCameraGrip: [0, 0, 255, 0.15], // rgba
  colorPreviewOverlay: [0, 0, 0, 0.4],
  colorPreviewHint: [255, 255, 255, 1],
  colorTooltip: [255, 255, 255, 1],
  colorTooltipFont: [0, 0, 0, 1],

  // defines how series should be sorted (order in tooltip & legend)
  // one of:
  //  * "maxAsc"
  //  * "maxDesc"
  //  * "minAsc"
  //  * "minDesc"
  //  * "medianAsc"
  //  * "medianDesc" (preferable)
  //  * "none"
  sortDataSetsBy: "medianDesc",

  // the following 3 settings define weights of content vs preview vs legend
  // sections
  layoutContentHeight: 5,
  layoutPreviewHeight: 1,
  layoutLegendHeight: 1.5,

  // palette to be used
  colorPalette: [
    // first 5 are color-blind friendly
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

  // long press duration in ms
  msLongPress: 500,

  // automatically switch to pseudo-log scale when charts take N-times more
  // vertical space.
  // pseudo-log scale means: log10(value - globalMinValue + 1000.0) - 3.0;
  // it allows to visualize negative numbers
  autoLogScaleThreshold: 15,

  // number of significant digits when fallen back to scientific notation:
  //  1.234e6
  expFmtSignificantDigits: 5,
};
