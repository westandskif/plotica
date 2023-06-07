const params = {
  // css selector of where to put the chart
  selector: "#chart-1",

  // optional content name (not used at the moment)
  contentName: "New chart",

  // one of 3 supported data types:
  //  * "date"
  //  * "datetime"
  //  * "number"
  coordType: "date",
  valueType: "number",

  // list of objects like:
  // {
  //     "name": name of a series
  //     "coords"; list of coordinates of coordType type
  //     "values": list of values of valueType type
  // }
  dataSets: [
    {
      name: "Foo",
      // all the below are valid dates
      coords: ["2020-01-01", 1577923200000, new Date("2020-01-03")],
      values: [10, "20", 30.0],
    },
  ],
};
