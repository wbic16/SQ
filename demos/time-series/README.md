# Time Series Datasets

Source: https://www.kaggle.com/datasets/shenba/time-series-datasets

There are four time-series example datasets to review.

## csv

The data provided by the Kaggle link above is provided to us in CSV (comma-separated values) format. In terms of space efficiency, CSV is better than JSON but worse than Phext.

* Shampoo Sales: 509 bytes for 36 records = 14 bytes/record
  - 499 bytes compressed = 13.9 bytes/record
* Beer Production: 6,903 bytes for 475 records = 15 bytes/record
  - 2,117 bytes compressed = 4.5 bytes/record

## json

* Shampoo Sales: 7,628 bytes for 36 records = 212 bytes/record
  - 687 bytes compressed = 19 bytes/record
* Beer Production: 85,156 bytes for 475 records = 179 bytes/record
  - 3,276 bytes compressed = 6.9 bytes/record

## phext

The shampoo sales example (a trivial 3-year sample) is incompressible. Both zip and 7z formats produce larger files. Beer production has more actual data and is neatly compressed.

* Shampoo Sales: 308 bytes for 36 records = 9 bytes/record
  - Incompressible!
* Beer Production: 4,702 bytes for 475 records = 10 bytes/record
  - 1,396 bytes compressed = 2.9 bytes/record