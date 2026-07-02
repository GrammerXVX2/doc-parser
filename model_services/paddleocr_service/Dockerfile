FROM python:3.11-slim

WORKDIR /workspace

COPY model_services/paddleocr_service/requirements.txt /tmp/requirements.txt
RUN pip install --no-cache-dir -r /tmp/requirements.txt

COPY . /workspace

EXPOSE 8101

CMD ["uvicorn", "model_services.paddleocr_service.app:app", "--host", "0.0.0.0", "--port", "8101"]
