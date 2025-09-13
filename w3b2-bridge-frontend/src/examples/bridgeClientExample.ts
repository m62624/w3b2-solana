import { Connection, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { BridgeClient } from '../services/bridgeClient';

/**
 * Пример использования BridgeClient
 */
export class BridgeClientExample {
  private client: BridgeClient;
  private connection: Connection;

  constructor(connection: Connection) {
    this.connection = connection;
    this.client = new BridgeClient(connection);
  }

  /**
   * Пример регистрации админа
   */
  async registerAdminExample() {
    console.log('=== Регистрация админа ===');
    
    // Создаем ключи
    const payer = Keypair.generate();
    const authority = Keypair.generate();
    const coSigner = Keypair.generate();

    // Получаем SOL для тестирования (в devnet)
    try {
      const signature = await this.connection.requestAirdrop(payer.publicKey, LAMPORTS_PER_SOL);
      await this.connection.confirmTransaction(signature);
      console.log('Получены тестовые SOL для payer');
    } catch (error) {
      console.error('Ошибка при получении airdrop:', error);
      return;
    }

    // Регистрируем админа
    try {
      const signature = await this.client.registerAdmin({
        payer: payer.publicKey,
        authority: authority.publicKey,
        coSigner: coSigner.publicKey,
        fundingAmount: 0.1 * LAMPORTS_PER_SOL, // 0.1 SOL
      }, [payer, authority, coSigner]);

      console.log('Админ зарегистрирован:', signature);
      
      // Получаем PDA админа
      const [adminPDA] = await this.client.getAdminAccountPDA(coSigner.publicKey);
      console.log('PDA админа:', adminPDA.toString());
      
      // Проверяем данные аккаунта
      const adminAccount = await this.client.getAdminAccount(adminPDA);
      console.log('Данные админа:', adminAccount);
      
    } catch (error) {
      console.error('Ошибка при регистрации админа:', error);
    }
  }

  /**
   * Пример регистрации пользователя
   */
  async registerUserExample() {
    console.log('=== Регистрация пользователя ===');
    
    const payer = Keypair.generate();
    const userWallet = Keypair.generate();
    const coSigner = Keypair.generate();

    // Получаем SOL для тестирования
    try {
      const signature = await this.connection.requestAirdrop(payer.publicKey, LAMPORTS_PER_SOL);
      await this.connection.confirmTransaction(signature);
      console.log('Получены тестовые SOL для payer');
    } catch (error) {
      console.error('Ошибка при получении airdrop:', error);
      return;
    }

    try {
      const signature = await this.client.registerUser({
        payer: payer.publicKey,
        userWallet: userWallet.publicKey,
        coSigner: coSigner.publicKey,
        initialBalance: 0.05 * LAMPORTS_PER_SOL, // 0.05 SOL
      }, [payer, userWallet, coSigner]);

      console.log('Пользователь зарегистрирован:', signature);
      
      // Получаем PDA пользователя
      const [userPDA] = await this.client.getUserAccountPDA(coSigner.publicKey);
      console.log('PDA пользователя:', userPDA.toString());
      
      // Проверяем данные аккаунта
      const userAccount = await this.client.getUserAccount(userPDA);
      console.log('Данные пользователя:', userAccount);
      
    } catch (error) {
      console.error('Ошибка при регистрации пользователя:', error);
    }
  }

  /**
   * Пример запроса финансирования
   */
  async requestFundingExample() {
    console.log('=== Запрос финансирования ===');
    
    const payer = Keypair.generate();
    const userWallet = Keypair.generate();
    const coSigner = Keypair.generate();
    const targetAdmin = Keypair.generate();

    // Получаем SOL для тестирования
    try {
      const signature = await this.connection.requestAirdrop(payer.publicKey, LAMPORTS_PER_SOL);
      await this.connection.confirmTransaction(signature);
      console.log('Получены тестовые SOL для payer');
    } catch (error) {
      console.error('Ошибка при получении airdrop:', error);
      return;
    }

    try {
      // Сначала регистрируем пользователя
      await this.client.registerUser({
        payer: payer.publicKey,
        userWallet: userWallet.publicKey,
        coSigner: coSigner.publicKey,
        initialBalance: 0,
      }, [payer, userWallet, coSigner]);

      console.log('Пользователь зарегистрирован для запроса финансирования');

      // Получаем PDA пользователя
      const [userPDA] = await this.client.getUserAccountPDA(coSigner.publicKey);

      // Создаем запрос на финансирование
      const fundingSignature = await this.client.requestFunding({
        payer: payer.publicKey,
        userAccount: userPDA,
        amount: 0.1 * LAMPORTS_PER_SOL,
        targetAdmin: targetAdmin.publicKey,
      }, [payer]);

      console.log('Запрос финансирования создан:', fundingSignature);
      
      // Получаем данные запроса
      const [fundingPDA] = await this.client.getFundingRequestPDA(userPDA, payer.publicKey);
      const fundingRequest = await this.client.getFundingRequest(fundingPDA);
      console.log('Данные запроса финансирования:', fundingRequest);
      
    } catch (error) {
      console.error('Ошибка при создании запроса финансирования:', error);
    }
  }


  /**
   * Запуск всех примеров
   */
  async runAllExamples() {
    console.log('Запуск примеров использования BridgeClient...\n');
    
    await this.registerAdminExample();
    console.log('\n');
    
    await this.registerUserExample();
    console.log('\n');
    
    await this.requestFundingExample();
    console.log('\n');
    
    console.log('Все примеры завершены!');
  }
}

// Функция для быстрого запуска примеров
export async function runBridgeClientExamples(connection: Connection) {
  const example = new BridgeClientExample(connection);
  await example.runAllExamples();
}
