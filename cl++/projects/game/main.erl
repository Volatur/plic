-module(main).
-export([start/0]).

start() ->
    try
        Options = [binary, {packet, 0}, {active, false}, {reuseaddr, true}],
    Listen_response = gen_tcp:listen(8080, Options),
    Socket = element(2, Listen_response),
    Accept_res = gen_tcp:accept(Socket),
    Client = element(2, Accept_res),
    gen_tcp:send(Client, "\e[2J\e[H"),
    gen_tcp:send(Client, unicode:characters_to_binary("Игра: Камень, ножницы, бумага\n")),
    gen_tcp:send(Client, unicode:characters_to_binary("Введи количество раундов:\n")),
    Rounds_response = gen_tcp:recv(Client, 0),
    Rounds_raw_data = element(2, Rounds_response),
    Round_bytes_size = byte_size(Rounds_raw_data) - 1,
    Cleaned_rounds_data = binary_part(Rounds_raw_data, 0, Round_bytes_size),
    Parsed_rounds_data = binary_to_list(Cleaned_rounds_data),
    Trimmed_rounds_data = string:trim(Parsed_rounds_data),
    Rounds = list_to_integer(Trimmed_rounds_data),
    lists:foreach(fun(I) ->
    clx_std:print(I)
end, lists:seq(1, Rounds)),
    gen_tcp:close(Client),
    gen_tcp:close(Socket)
    catch
        throw:{'__clx_return', ReturnValue} -> 
        ReturnValue
    end.