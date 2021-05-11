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

    # Hash the output
    hash_response = hash_output(response+"\n")

    print(f"Hash of the response: {hash_response}")


