defmodule Thulani.Bot.Consumer do
  use Nostrum.Consumer

  alias Nostrum.Api
  require Logger

  def start_link do
    Consumer.start_link(__MODULE__)
  end

  def handle_event({:MESSAGE_CREATE, {msg}, ws_state}, state) do
    case msg.content do
      "!thulani " <> command -> Logger.debug("got command", command: command)
      _ -> :ignore
    end

    {:ok, state}
  end

  def handle_event(_, state) do
    {:ok, state}
  end
end
