Cryptmon
========

A basic CLI app (for Linux and Mac currently, although other platforms might work) for displaying the price of Cryptocurrency coins,
and alerting based off price thresholds with some built-in alert/notification providers.

Made partly as a custom tool to scratch an itch, but also as a new project with which to learn and experiment with the Rust programming
language, as well as various HTTP/Web APIs for access to Crypto prices and alert/notification providers, and also to learn a bit more
about Crypto exchanges and price tracking.

It's still in the developmental phase (although not a high priority for me), but is functional to a fair degree, although it still has some rough
edges, and is very much designed with my limited use-cases in mind :)

Currently supported **Price Providers** (price data sources):

* [CoinGecko](https://www.coingecko.com/)
* [CryptoCompare](https://www.cryptocompare.com/) (note: sometime provides wrong prices in non-USD fiat currencies)
* [CoinMarketCap](https://coinmarketcap.com/) (API key needed)

Currently supported **Alert Providers**:

* SMTP email sending (not compiled in by default - feature needs to be enabled in Cargo.toml or on the cargo command line for building)
* [Textbelt](https://textbelt.com/) SMS sending (API key needed)
* [SimplePush](https://simplepush.io/) phone notifications (API key and SimplePush App needed)
* [PushSafer](https://www.pushsafer.com/) phone notifications (API key and PushSafer App needed)

in addition to built-in actions of `print`, which prints to the console, and `showNotification` which displays an OS alert.

a `cryptmon.ini` config file is needed (you can copy the `example_cryptmon_config.ini` example one as starting point) for configuration in the:

    $HOME/cryptmon.ini

or:

    $HOME/.config/cryptmon.ini

file path locations.

Building
--------

    cargo build --release

Should be all you need.

Running
-------

Currently there are two modes: **price view** (the default), which displays a table of coin prices in the terminal, based off the:
display.wantedCoins param in the `cryptmon.ini` config file, and **alerts**, which internally monitors prices for coins based off 
configured alerts in `cryptmon.ini`, and based off configured alert providers, will send alerts notifications if any of those trip.

To run the default price view, just run:

    ./cryptmon

From the directory where the executable is (target/release/).

This should show something like this, depending on your config params (and prices! :) ):

    Cryptmon Price View. Data last updated: 30/01 16:37:03

    Sym   Name      Price (NZD)  chng 24h  % chng 24h    low 24h   high 24h
    -----------------------------------------------------------------------
    BTC   Bitcoin     57,080.26    593.39       1.05%  56,217.59  57,252.36
    DOGE  Dogecoin       0.2141    0.0017       0.78%     0.2086     0.2176
    EOS   EOS             3.503    0.0370       1.07%      3.424      3.573
    ETH   Ethereum     3,972.99     67.68       1.73%   3,905.31   4,000.00
    LTC   Litecoin       164.79    0.8658       0.53%     161.85     167.52

To run with alerts active (must be configured in cryptmon.ini first):

    ./cryptmon alerts


Possible Future Work
--------------------

* More price providers
* More alert providers
* More configuration options
* More fine-grained alert threshold logic
* HTML price table output to file
* A GUI?
