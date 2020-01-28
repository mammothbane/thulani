import Config

config :thulani,
  env: Mix.env()

config :logger,
  level: :info

import_config "#{Mix.env}.exs"