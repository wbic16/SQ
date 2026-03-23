# Error Correction

One of the interesting side-effects of phext is the ability to include error-correction in the data stream.

Consider other plain text formats like XML or JSON. A single-byte error can irrevocably damage the contents, making parsing impossible. With phext, parsing only degrades.

We'll use the data files from the sparse relational sequence as a baseline.

## XML

* 28: there are 14 pairs of opening and closing tags in the data stream
* 8: four pairs of quotation marks
* 36/369 critical bytes = 9.8% of the data stream

## JSON

* 8: 4 pairs of opening/closing braces
* 2: one pair of square brackets
* 24: twelve quoted strings
* 34/314 critical bytes = 10.8% of the data stream

## Phext

* 5/179 critical bytes = 2.8% of the data stream

# Downstream Effects

Lossy channels affect XML and JSON at 3.5x higher rates. Consider the effect on parsing failures at scale.

It's also easier to patch phext documents with waypoints - the structure of an xml or json document degrades if it cannot be parsed - whereas a phext document could be restored given a known scroll coordinate.