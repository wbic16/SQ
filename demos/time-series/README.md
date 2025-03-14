# Time Series Datasets

Source: https://www.kaggle.com/datasets/shenba/time-series-datasets

There are four time-series example datasets to review.

## csv

The data provided by the Kaggle link above is provided to us in CSV (comma-separated values) format. In terms of space efficiency, CSV is better than JSON but worse than Phext.

* Shampoo Sales: 509 bytes for 36 records = 14 bytes/record
  - 499 bytes compressed = 13.9 bytes/record
* Beer Production: 6,903 bytes for 475 records = 15 bytes/record
  - 2,117 bytes compressed = 4.5 bytes/record
* Electric Production: 7,318 bytes for 397 records = 18.4 bytes/record
  - 2,624 bytes compressed = 6.6 bytes/record
* Daily Minimum Temperatures in ME: 55,852 bytes for 3,650 records = 15.3 bytes/record
  - 13,110 bytes compressed = 3.6 bytes/record

## json

* Shampoo Sales: 7,628 bytes for 36 records = 212 bytes/record
  - 687 bytes compressed = 19 bytes/record
* Beer Production: 85,156 bytes for 475 records = 179 bytes/record
  - 3,276 bytes compressed = 6.9 bytes/record
* Electric Production: 63,048 bytes for 397 records = 159 bytes/record
  - 3,878 bytes compressed = 9.8 bytes/record
* Daily Minimum Temperatures in ME: 673,750 bytes for 3,650 records = 184.6 bytes/record
  - 21,306 bytes compressed = 5.8 bytes/record

## phext

The shampoo sales example (a trivial 3-year sample) is incompressible. Both zip and 7z formats produce larger files. Beer production has more actual data and is neatly compressed.

* Shampoo Sales: 308 bytes for 36 records = 9 bytes/record
  - Incompressible!
* Beer Production: 4,702 bytes for 475 records = 10 bytes/record
  - 1,396 bytes compressed = 2.9 bytes/record
  - 2,754 bytes quined = 5.8 bytes/record
* Electric Production: 5,323 bytes for 397 records = 13.4 bytes/record
  - 1,800 bytes compressed = 4.5 bytes/record
  - 3,346 bytes quined = 8.4 bytes/record
* Daily Minimum Temperatures in ME: 28,206 bytes for 3,650 records = 7.7 bytes/record
  - 6,470 bytes compressed = 1.8 bytes/record

## Efficiency Comparison

The shampoo sales dataset is too small to draw any conclusions from. The beer and electric production datasets provide real-world examples over longer periods of time. We averaged the performance of CSV, JSON, and Phext for comparison.

  * CSV: 16.7 bytes/record live, 5.5 bytes/record compressed
  * JSON: 169 bytes/record live, 8.4 bytes/record compressed
  * Phext: 11.7 bytes/record live, 7.1 bytes/record quined, 3.7 bytes/record compressed

In the extreme, we see daily time series data is significantly more efficient in phext.

  * CSV: 3.6 bytes/record compressed
  * JSON: 5.8 bytes/record compressed
  * Phext: 1.8 bytes/record compressed