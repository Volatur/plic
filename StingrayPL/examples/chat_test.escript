#!/usr/bin/env escript
%% -*- erlang -*-
%%!
main(_) ->
    code:add_patha("examples"),
    io:format("=== Chat Test ===~n"),
    %% Connect to server
    case gen_tcp:connect({127,0,0,1}, 9999, [binary, {active, false}]) of
        {ok, Sock} ->
            io:format("Connected!~n"),
            %% Receive welcome
            case gen_tcp:recv(Sock, 0, 2000) of
                {ok, Welcome} -> io:format("Server: ~s~n", [binary_to_list(Welcome)]);
                _ -> ok
            end,
            %% Send message
            gen_tcp:send(Sock, <<"Hello from test client!\n">>),
            timer:sleep(500),
            io:format("Message sent.~n"),
            %% List users
            gen_tcp:send(Sock, <<"/list\n">>),
            case gen_tcp:recv(Sock, 0, 2000) of
                {ok, ListData} -> io:format("Server: ~s~n", [binary_to_list(ListData)]);
                _ -> ok
            end,
            %% Quit
            gen_tcp:send(Sock, <<"/quit\n">>),
            case gen_tcp:recv(Sock, 0, 2000) of
                {ok, ByeData} -> io:format("Server: ~s~n", [binary_to_list(ByeData)]);
                _ -> ok
            end,
            gen_tcp:close(Sock),
            io:format("Test complete!~n");
        {error, Reason} ->
            io:format("Connection failed: ~p~n", [Reason])
    end.
