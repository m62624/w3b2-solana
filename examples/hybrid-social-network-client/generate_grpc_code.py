import grpc_tools.protoc
import os

def generate_code():
    """
    Generates Python gRPC code from the .proto files in the ../../proto directory.
    """
    proto_path = '../../proto'
    output_path = './generated'

    if not os.path.exists(output_path):
        os.makedirs(output_path)

    # Ensure the generated directory is treated as a package
    with open(os.path.join(output_path, '__init__.py'), 'w') as f:
        pass

    proto_files = [os.path.join(proto_path, f) for f in os.listdir(proto_path) if f.endswith('.proto')]

    if not proto_files:
        print("No .proto files found.")
        return

    command = [
        'grpc_tools.protoc',
        '--proto_path={}'.format(proto_path),
        '--python_out={}'.format(output_path),
        '--grpc_python_out={}'.format(output_path),
    ] + proto_files

    print(f"Running command: {' '.join(command)}")

    exit_code = grpc_tools.protoc.main(command)

    if exit_code == 0:
        print("Successfully generated gRPC Python code.")
    else:
        print(f"Error generating gRPC Python code. Exit code: {exit_code}")

if __name__ == '__main__':
    generate_code()