from provider_agent import hash_output

output_string = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"0x5\"}\n"
print(f"output_string:{output_string}")
print(hash_output(output_string))