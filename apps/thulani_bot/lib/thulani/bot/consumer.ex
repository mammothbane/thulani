defmodule Thulani.Bot.Consumer do
  use Nostrum.Consumer

  alias Nostrum.Api
  require Logger

  def start_link do
    Consumer.start_link(__MODULE__)
  end

  def handle_event({:MESSAGE_CREATE, {msg}, _ws_state}, state) do
    case IO.inspect(msg.content) do
      "!thulani " <> command -> Logger.debug("got command", command: command)
      _ -> :ignore
    end

    {:ok, _} = Api.create_message(msg.channel.id, "sup")

    {:ok, state}
  end

  def handle_event(msg, state) do
    IO.inspect(msg)

    {:ok, state}
  end
end
