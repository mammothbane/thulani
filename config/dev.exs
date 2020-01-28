import Config

config :logger,
  level: :debug

config :nostrum,
  dev: true

IO.puts("hello")

Thulani.Bot.Config.load_env()
|> Enum.each(fn {key, vals} ->
  config key, vals
end)
