import Config

config :thulani_bot,
  env: Mix.env()

config :logger,
  level: :info

import_config "#{Mix.env()}.exs"
