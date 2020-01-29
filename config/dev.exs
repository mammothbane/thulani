import Config

config :logger,
  level: :debug

config :nostrum,
  dev: true

import_config "dotenv.exs"

Thulani.Config.Dotenv.load_dotenv!()
