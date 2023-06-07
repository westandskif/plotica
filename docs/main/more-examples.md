## When chart is small / too many data

1. the legend is paginated
1. press and hold a legend item to select the only one
1. click the only selected legend item to select all
1. the tooltip cuts off what it can't fit (_initial series sorting partially
   mitigates it_)

<script src="../../dist/index-iife.js"></script>
<script src="../themes/default.js"></script>

<div id="chart-1" style="max-width: 500px; height: 250px; margin: 0"></div>
<script src="../example-legend.js"></script>


## Automatic pseudo-log scale

When the difference in values is too big, pseudo-log scale may help you see
more:

<div id="chart-2" style="width: 100%; height: 60vh; margin: 0"></div>
<script src="../example-log.js"></script>

The pseudo-log operation is: `log10(value - globalMinValue + 1000.0) - 3.0`

Otherwise you would see the following:

<div id="chart-3" style="width: 100%; height: 60vh; margin: 0"></div>
<script src="../example-no-log.js"></script>
