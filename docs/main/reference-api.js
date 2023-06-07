const chartParams = {
  // ...
  // defines the data, see next sections
};
const chartConfig = {
  // ...
  // defines chart configuration, see next sections
};
// promise which resolves
let chartPromise = Graphima.createMain(params, chartConfig);

// destroy chart
chartPromise.then(function () {
  Graphima.destroyMain(chartId); // promise
});

// OPTIONAL: if you want to run all the initialization code before createMain
// to minimize latency of the first call
Graphima.init(); // promise
