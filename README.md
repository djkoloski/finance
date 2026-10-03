# `finance`

A financial report generator.

I made this so that I could do better at keeping track of my finances, and
understanding how much income I have as well as how much I'm paying for various
goods and services. I also added a retirement calculator so I could better
understand the value of investing and how much investment I need before I can
retire comfortably.

## Example output

![summary](https://github.com/djkoloski/finance/raw/main/example/03-2026_output/summary.png "Summary example")
![income](https://github.com/djkoloski/finance/raw/main/example/03-2026_output/income.png "Income example")
![investments](https://github.com/djkoloski/finance/raw/main/example/03-2026_output/investments.png "Investments example")

See the [sample PDF](https://github.com/djkoloski/finance/raw/main/example/03-2026_output/report.pdf) for an example of rendered output.

## Features

Generated financial reports include:

- Total income and spending
- Net savings, and average net savings per month
- Spending-to-income ratio and individual-to-shared expenses ratio
- Categorical breakdowns for income and spending
- Comparisons between the actual income and spend for the last month, and the
  expected income and spend based on the year prior to last month.
- Change calculation for observing how your monthly budgeting changed from the
  last month of transactions.
- Investments tracking and rate-of-return calculation.
- Retirement projection given your current savings rate, and broken down for
  anywhere from 4-10% APY. Separately reports retirement based on brokerage-only
  investments (which should not include 401ks or other age-restricted investment
  vehicles), versus all investments (which include them).
- All transactions for the month categorized and sorted by date.

## Usage

```
$ ./finance --help

Finance report generator

Usage: finance [OPTIONS] <DATA_NAME>

Arguments:
  <DATA_NAME>
          The name of the data directory adjacent to the configuration file

Options:
  -c, --config <CONFIG>
          The path to the configuration file
          
          [default: data/config.json]

  -f, --format <FORMAT>
          The output format

          Possible values:
          - json: JSON formatting
          - html: HTML formatting
          
          [default: html]

  -o, --output <OUTPUT>
          The output file

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## How to generate a report

See the `example` directory for example input and output.

1. Export your account transactions for the past 13+ months as a CSV. Repeat for
   each account to include in the report (e.g. checking, savings, joint). CSV
   rows should be `Date, Time, Amount, Type, Description`, where `Date` is
   formatted `YYYY-MM-DD`, `Time` is formatted `HH:MM:SS`, and `Type` is either
   `"Withdrawal"` or `"Deposit"`.
2. Create a `meta.json` with information about the specific period being
   reported. See `src/data.rs` for the schema.
3. Create a `config.json` file with your financial information. See
   `src/config.rs` for the schema. The most important part of the configuration
   is `"categories"`, which defines the transaction categories and provides
   regex matchers for transaction descriptions.
4. Run the report generator by choosing a config file, data directory, output
   format, and destination file. By default, HTML reports will be generated and
   output to `stdout` if no output file is specified.

How you choose to categorize transactions is up to you, but the report generator
has four groups of categories that are essential for some calculations:

- `income` is any source of income or adjustments to income. Put wages,
  interest, etc in this group.
- `shared_expenses` are shared cost-of-living expenses. This might include
  categories like "utilities", "insurance", "rent". You may also want to include
  categories like "streaming" and "groceries" if you share the expenses for
  those with others.
- `individual_expenses` are expenses that you are personally responsible for.
  These are usually discretionary, but should be used for any expenses that are
  not shared with others.
- `internal` are transactions that should be ignored when calculating income and
  expenses. For example, internal transfers should be ignored because they are
  neither income nor expenses.
