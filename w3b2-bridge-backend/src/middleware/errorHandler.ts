import { Request, Response, NextFunction } from 'express';
import { ApiResponse } from '../types/index';

export function errorHandler(error: Error, req: Request, res: Response, next: NextFunction): void {
  console.error('❌ Ошибка:', error);

  // Определяем тип ошибки и соответствующий HTTP статус
  let statusCode = 500;
  let message = 'Внутренняя ошибка сервера';

  if (error.name === 'ValidationError') {
    statusCode = 400;
    message = 'Ошибка валидации данных';
  } else if (error.name === 'UnauthorizedError') {
    statusCode = 401;
    message = 'Неавторизованный доступ';
  } else if (error.name === 'ForbiddenError') {
    statusCode = 403;
    message = 'Доступ запрещен';
  } else if (error.name === 'NotFoundError') {
    statusCode = 404;
    message = 'Ресурс не найден';
  } else if (error.name === 'ConflictError') {
    statusCode = 409;
    message = 'Конфликт данных';
  } else if (error.message && error.message.includes('Invalid public key')) {
    statusCode = 400;
    message = 'Неверный публичный ключ';
  } else if (error.message && error.message.includes('Insufficient funds')) {
    statusCode = 400;
    message = 'Недостаточно средств';
  } else if (error.message && error.message.includes('Transaction failed')) {
    statusCode = 400;
    message = 'Ошибка транзакции';
  }

  const response: ApiResponse = {
    success: false,
    error: message,
    timestamp: new Date().toISOString(),
  };

  res.status(statusCode).json(response);
}

// Middleware для обработки 404 ошибок
export function notFoundHandler(req: Request, res: Response, next: NextFunction): void {
  const response: ApiResponse = {
    success: false,
    error: `Маршрут ${req.method} ${req.path} не найден`,
    timestamp: new Date().toISOString(),
  };

  res.status(404).json(response);
}

// Middleware для валидации публичного ключа
export function validatePublicKey(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  const { publicKey } = req.params;

  if (!publicKey) {
    return next(new Error('Публичный ключ обязателен'));
  }

  try {
    new (require('@solana/web3.js').PublicKey)(publicKey);
    next();
  } catch (error) {
    console.error('❌ Ошибка валидации публичного ключа:', error);
    next(new Error('Неверный формат публичного ключа'));
  }
}

// Middleware для логирования запросов
export function requestLogger(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  const start = Date.now();

  res.on('finish', () => {
    const duration = Date.now() - start;
    console.log(
      `${req.method} ${req.path} - ${res.statusCode} - ${duration}ms`
    );
  });

  next();
}
