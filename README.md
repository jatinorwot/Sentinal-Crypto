Rust WebSocket Client and Aggregator


A Rust application for retrieving data from a WebSocket server, performing aggregation, and validating signatures using multiple client processes and an aggregator process.


Features


WebSocket client processes fetch real-time data from a specified server and calculate average prices.


Aggregator process collects data from client processes, validates signatures, and calculates the final average.


Supports configurable number of client processes and aggregation duration.


Utilizes Tokio for asynchronous I/O and concurrency management.


Implements signature validation and data aggregation using Tokio channels.

Usage


The WebSocket client and aggregator application supports the following command-line options:


Usage: ./simple --mode=cache OR ./simple --mode=read



Options:


  --mode=cache            Run in cache mode to fetch data from WebSocket server and calculate average prices.

  
  --mode=read             Run in read mode to read aggregated data from a file.
