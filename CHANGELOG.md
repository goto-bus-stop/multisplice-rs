# multisplice-rs change log

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](http://semver.org/).

## 0.3.0
* Return `Cow` instances from `slice()` methods to avoid unnecessary copies.

## 0.2.0
* Add slice and splice methods that use Ranges instead of indices.
* Accept `Cow` instances in `splice()` methods.

## 0.1.0
* Initial release.
