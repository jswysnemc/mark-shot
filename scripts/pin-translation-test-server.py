"""【贴图翻译回归】【回环服务】返回受控翻译响应，全部内容为测试文本，不使用外部服务或真实凭据"""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import threading


class Service:
    """【贴图翻译回归】【服务状态】保存实际请求，支持失败、空结果和延迟成功"""

    def __init__(self):
        """【贴图翻译回归】【私有服务】无参数，启动回环HTTP监听并返回服务实例"""
        self.mode = "failure"
        self.requests = []
        self.release = threading.Event()
        owner = self

        class Handler(BaseHTTPRequestHandler):
            """【贴图翻译回归】【真实协议】解析生产请求并生成对应OpenAI响应"""

            def do_POST(self):
                """【贴图翻译回归】【响应选择】无参数，记录分段与语言，按模式写入响应，无返回值"""
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                prompt = json.loads(body["messages"][-1]["content"])
                owner.requests.append(prompt)
                mode = owner.mode
                if mode == "delay":
                    owner.release.wait(8)
                status = 503 if mode == "failure" else 200
                translations = [] if mode == "empty" else [{"id": segment["id"], "text": "Translated sample text"} for segment in prompt["segments"]]
                response = {"error": {"message": "Translation fixture failed"}} if status == 503 else {"choices": [{"message": {"content": json.dumps({"translations": translations})}}]}
                encoded = json.dumps(response).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(encoded)))
                self.end_headers()
                try:
                    self.wfile.write(encoded)
                except (BrokenPipeError, ConnectionResetError):
                    pass

            def log_message(self, *_arguments):
                """【贴图翻译回归】【日志隔离】忽略HTTP访问日志，不输出请求正文，无返回值"""

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.endpoint = f"http://127.0.0.1:{self.server.server_port}/v1"

    def close(self):
        """【贴图翻译回归】【服务回收】无参数，释放延迟请求并关闭所属监听，无返回值"""
        self.release.set()
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=3)
