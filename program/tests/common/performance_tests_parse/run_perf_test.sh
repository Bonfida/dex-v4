#!/bin/bash

cargo test -- perf --nocapture &> tests/common/performance_tests_parse/out.log
cd  tests/common/performance_tests_parse/
python3 parse_log.py

