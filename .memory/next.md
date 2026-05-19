# 下一步任务

执行 `.prompt/008-container-deployment.md`：

1. 新增后端多阶段 Dockerfile。
2. 新增 Web 容器构建方案或静态资源服务容器。
3. 新增 docker-compose 基础部署。
4. 新增 `deploy/k8s/` Deployment / Service / ConfigMap 基础 YAML。
5. 提供容器监听 `0.0.0.0` 示例配置。
6. 明确 Worker 只主动出站连接 Worker Tunnel，不暴露入站端口。
7. 按 cargo、maven、bun 既有命令全量验证；Docker 可用时验证 build/compose。
8. 更新设计路线图、`.memory` 和后续 `.prompt`。
9. 提交并推送。
