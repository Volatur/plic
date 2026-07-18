-module(clear_input).
-export([clear_input/1]).

clear_input(Client) ->
    try
        Response = gen_tcp:recv(Client, 0),
    Raw_data = clx_std:get_element(Response, 2),
    Bytes_size = byte_size(Raw_data) - 1,
    Cleaned_data = binary_part(Raw_data, 0, Bytes_size),
    Parsed_data = binary_to_list(Cleaned_data),
    Trimmed_data = string:trim(Parsed_data),
    throw({'__clx_return', Trimmed_data})
    catch
        throw:{'__clx_return', ReturnValue} -> 
        ReturnValue
    end.