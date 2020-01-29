defmodule Thulani.Bot.Application do
  alias Thulani.Bot.Config
  use Application

  @applications [
    :nostrum
  ]

  def start(_type, _args) do
    Config.init!()

    Enum.each(@applications, fn a -> Application.start(a) end)

    children = []

    opts = [strategy: :one_for_one, name: Thulani.Bot.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
