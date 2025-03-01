Sparse Spontaneous Events
-------------------------

This demo showcases the power of implicit structure - demonstrating how phext can improve data processing. For this data set, phext is the only format to beat both zip and 7z in terms of compression. It also happens to remain readable as a plain multi-dimensional text format to boot! (Toot, toot!)

File System
-----------
Using the file system, we run into massive inefficiency - every file consumes 4 KB, so we've consumed 100x the disk space required by our dataset.

- Messages Encoded: 12
- Files: 13
- Data Structure: each file encodes the user and timestamp in the filename, and the message as file content
- Zip Compression: 3,116 bytes
- 7z Compression: 760 bytes
- Uncompressed Size: 481 bytes (file names are 'free')
- Disk Space Per Message: 4 KB
- Live DB: 250 million messages/TB
- Archive: 17.4 billion messages/TB

JSON
----
Using JSON, we can encode our events as single files. We send the messages we have on hand in each wave.

- Messages Encoded: 12
- Files: 3
- Data Structure: each entry has three fields: user, timestamp, and message
- Zip Compression: 1,295 bytes
- 7z Compression: 719 bytes
- Uncompressed Size: 1,908 bytes
- Disk Space Per Message: 1 KB
- Live DB: 1 billion messages/TB
- Archive: 18.4 billion messages/TB

Monolithic JSON
---------------
OK, so using the file system was a worse idea. What happens if we stuff all of our events into one JSON file?

- Messages Encoded: 12
- Files: 1
- Data Structure: entries are just concatenated as they arrive
- Zip Compression: 811 bytes
- 7z Compression: 697 bytes
- Uncompressed Size: 1,862 bytes
- Disk Space Per Message: 341 bytes
- Live DB: 7 billion messages/TB
- Archive: 19 billion messages/TB


Tab-Delimited
-------------
The problem with monolithic-json is that it doesn't feel like information we can use directly. Maybe a tab-delimited file will work better? We'll get some space savings on the live DB by keeping a user lookup table. It complicates the format, but seems to be worth it - we're up to 15B live messages per TB!

- Messages Encoded: 12
- Files: 1
- Data Structure: entries are rows in a table
- Zip Compression: 748 bytes
- 7z Compression: 661 bytes
- Uncompressed Size: 869 bytes
- Disk Space Per Message: 80 bytes
- Live DB: 15 billion messages/TB
- Archive: 20 billion messages/TB

Phext
-----
Phext enables us to run our live database as an archive - because the format is already space-time efficient. Relative to the file system example, we've only consumed an extra 78 bytes - most of which were spent on the user table in scroll 1.1.1/1.1.1/1.1.1.

In this example, we've defined a coordinate mapping that encodes the date as follows.

* Library = Year
* Shelf = Month
* Series = Day
* Collection = Hour
* Volume = Minute
* Book = User ID
* Chapter = 1 (Reserved)
* Section = 1 (Reserved)
* Scroll = 1 (Reserved)

When inserting content into our phext dataset, we can now choose our index from the timestamp. Comparing our approach to the others yields impressive benefits. Note that phext consumers are encouraged by the format to make critical optimizations (such as adding a user lookup table), because of the pressure to produce content coordinates. You might argue that the tab-delimited approach could also make use of a user lookup table, but where are you going to store it? You need a way to encode the relationship in a standardized way! (Enter, Phext...)

- Messages Encoded: 12
- Files: 1
- Data Structure: hierarchical plain text
- Zip Compression: 575 bytes (larger if compressed)
- 7z Compression: 566 bytes
- Uncompressed Size: 575 bytes
- Disk Space Per Message: 46 bytes
- Live DB: 23.6 billion messages/TB
- Archive: 23.6 billion messages/TB