import hashlib


# Hash the output string
def hash_output(output_string):
    return hashlib.sha3_256(output_string.encode()).hexdigest()

if __name__ == "__main__":
    # Get result frorm block number
    # url = "https://dev-gateway.massbit.io/bsc"
    # method = "eth_getBlockTransactionCountByNumber"
    # block_number = input("input block_number")
    # result = get_job_result(url,method,block_number)

    response = input("Input the response form worker for hashing:")
    url = input("Input the worker url:")
    block_number = input("Input the block_number:")

    # Hash the output
    hash_response = hash_output(response+"\n")

    job_input = "{\"url\":\"http://" + url + "/\",\"method\":\"eth_getBlockTransactionCountByNumber\",\"block_number\":\"" + block_number + "\",\"job_proposal\":\"0\"}"

    # Print job_input for report
    print(f"job_input: {job_input}")
    print(f"job_output: {hash_response}")


