-module(main).
-export([start/0]).









start() ->
    try
        Db = db:db(),
    Serv = server:server(),
    Accept_response = gen_tcp:accept(Serv),
    Client = clx_std:get_element(Accept_response, 2),
    clear_screen:clear_screen(Client),
    (fun Loop() ->
        case clx_std:to_boolean(true) of
            true ->
                auth:auth(Client, Db),
                Loop();
            _ ->
                ok
        end
    end)()
    catch
        throw:{'__clx_return', ReturnValue} -> 
        ReturnValue
    end.