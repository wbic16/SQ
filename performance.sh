#!/bin/sh
rm -rf .sq
ps -ef |grep sq |grep -v grep |awk '{print $2}' |xargs kill -9
./target/release/sq the-odyssey.phext &
sleep 0
INDEX=1
while [ $INDEX -lt 51 ]; do
  ./target/release/sq select "1.1.1/1.1.1/3.1.$INDEX"
  INDEX=$(($INDEX + 1))
done
