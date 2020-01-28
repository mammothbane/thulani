def load_dotenv!() do
  {:ok, projroot} = __DIR__ |> get_projroot

  contents =
    case projroot |> Path.join(".env") |> File.read() do
      {:error, :enoent} ->
        Logger.warn("skipping dotenv file: doesn't exist")
        ""

      {:ok, contents} ->
        contents
    end

  contents
  |> String.split("\n")
  |> Enum.map(&String.trim/1)
  |> Enum.filter(fn x -> x != "" end)
  |> Enum.map(fn x ->
    result = String.split(x, "=")
    {Enum.at(result, 0), Enum.at(result, 1)}
  end)
  |> System.put_env()
end

def get_projroot(base) do
  result =
    with {:ok, info} <- base |> Path.join(".thulani_root") |> File.stat(),
         :regular <- info.type,
         do: {:ok, base}

  case result do
    {:error, :enoent} -> get_projroot(base |> Path.join("..") |> Path.expand())
    {:ok, "/"} -> {:error, "couldn't find .thulani_root"}
    x -> x
  end
end
