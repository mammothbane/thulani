import Config

config :logger,
  level: :debug

import_config "dotenv.exs"

Thulani.Config.Dotenv.load_dotenv!()
