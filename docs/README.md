# Main chart

<figure markdown>

![Image title](images/logo-black.png){ align=left width=128}

<p style="text-align: left">
<strong>Graphima</strong> (<i>Greek: γράφημα</i>) is an attempt to build
full-featured WebAssembly-based monolith charts.
</p>

</figure>

## Tutorial

1. select an area within the main or the preview chart to zoom in
1. drag the camera within the preview chart (when zoomed in)
1. click the preview part to zoom out
1. click the main chart to freeze the tooltip (click once again to undo)
1. press and hold a legend item to select the series and deselect everything
   else
1. click the only selected legend item to select all

<div id="chart-1" style="width: 100%; height: 60vh; margin: 0"></div>
<script src="dist/index-iife.js"></script>
<script src="readme.js"></script>

#### Nice bits

- default theme uses color-blind friendly palette (_5 colors_)
- automatic pseudo-log scale (`log10(value - globalMinValue + 1000.0) - 3.0`)
- proposed config defaults to sorting series by median in descending order
  (_the first in the tooltip is the one with highest median_)
- mobile friendly (_+supports pinch gesture_)
- tolerant to too many series (paginated legend + tooltip with max size)

## Installation

#### As IIFE module

`iife` module (_Immediate Invoked Function Expression_) makes it available as
`window.Graphima`.

```js title="in index.html above your app code"
--8<-- "docs/import-iife.html"
```

#### As ESM module

Run `npm i graphima` and then use a bundler of your choice (e.g.
[Rollup](https://rollupjs.org)):

```js
--8<-- "docs/import-es.js"
```

## Code of the chart above

```js
--8<-- "docs/readme.js"
```
